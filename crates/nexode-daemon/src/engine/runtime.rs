use super::*;

#[derive(Debug)]
pub(super) struct RuntimeState {
    pub(super) session_budget_max_usd: f64,
    pub(super) total_session_cost: f64,
    pub(super) last_event_sequence: u64,
    pub(super) projects: BTreeMap<String, ProjectRuntime>,
    pub(super) slot_project: BTreeMap<String, String>,
}

impl RuntimeState {
    pub(super) fn from_session(
        session: SessionConfig,
        verification_timeout: Duration,
    ) -> Result<Self, DaemonError> {
        let mut projects = BTreeMap::new();
        let mut slot_project = BTreeMap::new();

        for project in session.projects {
            let repo_path = project
                .repo
                .clone()
                .ok_or_else(|| DaemonError::MissingRepository {
                    project_id: project.id.clone(),
                })?;
            let orchestrator = GitWorktreeOrchestrator::with_worktree_root_and_timeout(
                &repo_path,
                default_worktree_root(&repo_path),
                verification_timeout,
            )?;
            let mut slots = BTreeMap::new();

            for slot in &project.slots {
                if slot_project
                    .insert(slot.id.clone(), project.id.clone())
                    .is_some()
                {
                    return Err(DaemonError::DuplicateSlotId {
                        slot_id: slot.id.clone(),
                    });
                }
                slots.insert(slot.id.clone(), SlotRuntime::from_slot(slot.clone()));
            }

            projects.insert(
                project.id.clone(),
                ProjectRuntime::from_config(project, repo_path, orchestrator, slots),
            );
        }

        Ok(Self {
            session_budget_max_usd: session.session.budget.max_usd.unwrap_or_default(),
            total_session_cost: 0.0,
            last_event_sequence: 0,
            projects,
            slot_project,
        })
    }

    pub(super) fn from_recovered_session(
        session: SessionConfig,
        verification_timeout: Duration,
        persisted: &PersistedRuntimeState,
    ) -> Result<Self, DaemonError> {
        let mut state = Self::from_session(session, verification_timeout)?;
        state.total_session_cost = persisted.total_session_cost;

        for (project_id, persisted_project) in &persisted.projects {
            let Some(project) = state.projects.get_mut(project_id) else {
                continue;
            };
            project.current_cost_usd = persisted_project.current_cost_usd;
            project.merge_queue = persisted_project
                .merge_queue
                .iter()
                .filter(|slot_id| project.slots.contains_key(*slot_id))
                .cloned()
                .collect();
            project.merge_inflight_slot = persisted_project
                .merge_inflight_slot
                .as_ref()
                .filter(|slot_id| project.slots.contains_key(*slot_id))
                .cloned();

            for (slot_id, persisted_slot) in &persisted_project.slots {
                let Some(slot) = project.slots.get_mut(slot_id) else {
                    continue;
                };
                if let Some(task) = persisted_slot.task.clone() {
                    slot.task = task;
                }
                if let Some(mode) = persisted_slot
                    .mode
                    .and_then(|raw| AgentMode::try_from(raw).ok())
                {
                    slot.mode = mode;
                }
                if let Some(status) = persisted_slot
                    .task_status
                    .and_then(|raw| TaskStatus::try_from(raw).ok())
                {
                    slot.task_status = status;
                }
                slot.current_agent_id = persisted_slot.current_agent_id.clone();
                slot.current_agent_pid = persisted_slot.current_agent_pid;
                slot.worktree_path = persisted_slot.worktree_path.as_ref().map(PathBuf::from);
                slot.total_tokens = persisted_slot.total_tokens;
                slot.total_cost_usd = persisted_slot.total_cost_usd;
            }
        }

        Ok(state)
    }

    pub(super) fn to_persisted(&self) -> PersistedRuntimeState {
        PersistedRuntimeState {
            total_session_cost: self.total_session_cost,
            projects: self
                .projects
                .iter()
                .map(|(project_id, project)| {
                    (
                        project_id.clone(),
                        PersistedProjectState {
                            current_cost_usd: project.current_cost_usd,
                            merge_queue: project.merge_queue.clone(),
                            merge_inflight_slot: project.merge_inflight_slot.clone(),
                            slots: project
                                .slots
                                .iter()
                                .map(|(slot_id, slot)| {
                                    (
                                        slot_id.clone(),
                                        PersistedSlotState {
                                            task: Some(slot.task.clone()),
                                            mode: Some(slot.mode as i32),
                                            task_status: Some(slot.task_status as i32),
                                            current_agent_id: slot.current_agent_id.clone(),
                                            current_agent_pid: slot.current_agent_pid,
                                            worktree_path: slot
                                                .worktree_path
                                                .as_ref()
                                                .map(|path| path.display().to_string()),
                                            total_tokens: slot.total_tokens,
                                            total_cost_usd: slot.total_cost_usd,
                                        },
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub(super) fn slot_ids(&self) -> Vec<String> {
        self.slot_project.keys().cloned().collect()
    }

    pub(super) fn project_slot_ids(&self, project_id: &str) -> Vec<String> {
        self.projects
            .get(project_id)
            .map(|project| project.slots.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(super) fn snapshot(&self) -> FullStateSnapshot {
        FullStateSnapshot {
            projects: self
                .projects
                .values()
                .map(ProjectRuntime::snapshot)
                .collect(),
            task_dag: self
                .projects
                .values()
                .flat_map(|project| project.task_nodes())
                .collect(),
            total_session_cost: self.total_session_cost,
            session_budget_max_usd: self.session_budget_max_usd,
            last_event_sequence: self.last_event_sequence,
        }
    }
}

#[derive(Debug)]
pub(super) struct ProjectRuntime {
    pub(super) id: String,
    pub(super) display_name: String,
    pub(super) repo_path: PathBuf,
    pub(super) color: String,
    pub(super) tags: Vec<String>,
    pub(super) budget: BudgetConfig,
    pub(super) verify: Option<VerifyConfig>,
    pub(super) current_cost_usd: f64,
    pub(super) merge_queue: VecDeque<String>,
    pub(super) merge_inflight_slot: Option<String>,
    pub(super) orchestrator: GitWorktreeOrchestrator,
    pub(super) slots: BTreeMap<String, SlotRuntime>,
}

impl ProjectRuntime {
    fn from_config(
        config: ProjectConfig,
        repo_path: PathBuf,
        orchestrator: GitWorktreeOrchestrator,
        slots: BTreeMap<String, SlotRuntime>,
    ) -> Self {
        Self {
            id: config.id,
            display_name: config.display_name,
            repo_path,
            color: config.color.unwrap_or_default(),
            tags: config.tags,
            budget: config.budget,
            verify: config.verify,
            current_cost_usd: 0.0,
            merge_queue: VecDeque::new(),
            merge_inflight_slot: None,
            orchestrator,
            slots,
        }
    }

    fn snapshot(&self) -> Project {
        Project {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            repo_path: self.repo_path.display().to_string(),
            color: self.color.clone(),
            tags: self.tags.clone(),
            budget_max_usd: self.budget.max_usd.unwrap_or_default(),
            budget_warn_usd: self.budget.warn_usd.unwrap_or_default(),
            current_cost_usd: self.current_cost_usd,
            slots: self
                .slots
                .values()
                .map(|slot| slot.snapshot(&self.id))
                .collect(),
        }
    }

    fn task_nodes(&self) -> Vec<TaskNode> {
        self.slots
            .values()
            .map(|slot| TaskNode {
                id: slot.id.clone(),
                title: slot.task.clone(),
                description: slot.task.clone(),
                status: slot.task_status as i32,
                assigned_agent_id: slot.current_agent_id.clone().unwrap_or_default(),
                project_id: self.id.clone(),
                dependency_ids: Vec::new(),
            })
            .collect()
    }
}

#[derive(Debug)]
pub(super) struct SlotRuntime {
    pub(super) id: String,
    pub(super) task: String,
    pub(super) model: String,
    pub(super) harness: Option<String>,
    pub(super) mode: AgentMode,
    pub(super) branch: String,
    pub(super) timeout_minutes: u64,
    pub(super) provider_config: BTreeMap<String, String>,
    pub(super) context: ContextConfig,
    pub(super) task_status: TaskStatus,
    // DECISION: keep pause history in memory until WAL/checkpoint versioning exists.
    pub(super) pre_pause_status: Option<TaskStatus>,
    pub(super) current_agent_id: Option<String>,
    pub(super) current_agent_pid: Option<u32>,
    pub(super) worktree_path: Option<PathBuf>,
    pub(super) total_tokens: u64,
    pub(super) total_cost_usd: f64,
    pub(super) pending_swap_from: Option<String>,
    pub(super) supervisor: Option<SlotSupervisor>,
}

impl SlotRuntime {
    fn from_slot(slot: SlotConfig) -> Self {
        Self {
            id: slot.id,
            task: slot.task,
            model: slot.model,
            harness: slot.harness,
            mode: slot.mode,
            branch: slot.branch,
            timeout_minutes: slot.timeout_minutes.max(1),
            provider_config: slot.provider_config,
            context: slot.context,
            task_status: TaskStatus::Pending,
            pre_pause_status: None,
            current_agent_id: None,
            current_agent_pid: None,
            worktree_path: None,
            total_tokens: 0,
            total_cost_usd: 0.0,
            pending_swap_from: None,
            supervisor: None,
        }
    }

    fn snapshot(&self, project_id: &str) -> AgentSlot {
        AgentSlot {
            id: self.id.clone(),
            project_id: project_id.to_string(),
            task: self.task.clone(),
            mode: self.mode as i32,
            branch: self.branch.clone(),
            current_agent_id: self.current_agent_id.clone().unwrap_or_default(),
            worktree_id: self
                .worktree_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            total_tokens: self.total_tokens,
            total_cost_usd: self.total_cost_usd,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SlotDescriptor {
    pub(super) project_id: String,
    pub(super) slot_id: String,
    pub(super) repo_path: PathBuf,
    pub(super) branch: String,
    pub(super) task: String,
    pub(super) model: String,
    pub(super) harness: Option<String>,
    pub(super) mode: AgentMode,
    pub(super) timeout_minutes: u64,
    pub(super) provider_config: BTreeMap<String, String>,
    pub(super) context: ContextConfig,
    pub(super) budget: BudgetConfig,
    pub(super) orchestrator: GitWorktreeOrchestrator,
    pub(super) verify: Option<VerifyConfig>,
}

#[derive(Debug, Clone)]
pub(super) struct MergeDescriptor {
    pub(super) orchestrator: GitWorktreeOrchestrator,
    pub(super) worktree_path: PathBuf,
    pub(super) verify: Option<VerifyConfig>,
}

pub(super) fn resolve_accounting_path(session_path: &Path, requested_path: &Path) -> PathBuf {
    if requested_path.is_absolute() {
        return requested_path.to_path_buf();
    }

    session_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(requested_path)
}

fn default_worktree_root(repo_path: &Path) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_string());
    repo_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".nexode-worktrees")
        .join(repo_name)
}

impl DaemonEngine {
    pub(super) fn slot(&self, slot_id: &str) -> Option<&SlotRuntime> {
        let project_id = self.state.slot_project.get(slot_id)?;
        self.state.projects.get(project_id)?.slots.get(slot_id)
    }

    pub(super) fn slot_descriptor(&self, slot_id: &str) -> Option<SlotDescriptor> {
        let project_id = self.state.slot_project.get(slot_id)?;
        let project = self.state.projects.get(project_id)?;
        let slot = project.slots.get(slot_id)?;
        Some(SlotDescriptor {
            project_id: project_id.clone(),
            slot_id: slot_id.to_string(),
            repo_path: project.repo_path.clone(),
            branch: slot.branch.clone(),
            task: slot.task.clone(),
            model: slot.model.clone(),
            harness: slot.harness.clone(),
            mode: slot.mode,
            timeout_minutes: slot.timeout_minutes.max(1),
            provider_config: slot.provider_config.clone(),
            context: slot.context.clone(),
            budget: project.budget.clone(),
            orchestrator: project.orchestrator.clone(),
            verify: project.verify.clone(),
        })
    }

    pub(super) fn merge_descriptor(&self, slot_id: &str) -> Option<MergeDescriptor> {
        let project_id = self.state.slot_project.get(slot_id)?;
        let project = self.state.projects.get(project_id)?;
        let slot = project.slots.get(slot_id)?;
        Some(MergeDescriptor {
            orchestrator: project.orchestrator.clone(),
            worktree_path: slot.worktree_path.clone()?,
            verify: project.verify.clone(),
        })
    }

    pub(super) fn slot_mut(&mut self, slot_id: &str) -> Option<&mut SlotRuntime> {
        let project_id = self.state.slot_project.get(slot_id)?.clone();
        self.state
            .projects
            .get_mut(&project_id)?
            .slots
            .get_mut(slot_id)
    }

    pub(super) fn pre_pause_status(&self, slot_id: &str) -> Option<TaskStatus> {
        self.slot(slot_id).and_then(|slot| slot.pre_pause_status)
    }

    pub(super) fn find_slot_by_agent(&self, agent_id: &str) -> Option<String> {
        self.state.projects.iter().find_map(|(_, project)| {
            project.slots.iter().find_map(|(slot_id, slot)| {
                (slot.current_agent_id.as_deref() == Some(agent_id)).then(|| slot_id.clone())
            })
        })
    }

    pub(super) fn current_task_status(&self, slot_id: &str) -> Option<TaskStatus> {
        self.slot(slot_id).map(|slot| slot.task_status)
    }
}
