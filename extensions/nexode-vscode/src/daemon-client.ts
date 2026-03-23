import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import * as vscode from 'vscode';

import {
  AgentOutputLine,
  CommandResponse,
  ConnectionStatus,
  Emitter,
  FullStateSnapshot,
  HypervisorEvent,
  normalizeCommandResponse,
  normalizeEvent,
  normalizeSnapshot,
} from './state';

const CLIENT_VERSION = '0.0.1';
const INITIAL_RECONNECT_DELAY_MS = 2_000;
const MAX_RECONNECT_DELAY_MS = 30_000;
const READY_TIMEOUT_MS = 5_000;

interface HypervisorClientConstructor {
  new (
    address: string,
    credentials: grpc.ChannelCredentials,
    options?: Record<string, unknown>,
  ): HypervisorServiceClient;
}

interface HypervisorServiceClient extends grpc.Client {
  getFullState(
    request: Record<string, never>,
    callback: (
      error: grpc.ServiceError | null,
      response: Record<string, unknown> | undefined,
    ) => void,
  ): grpc.ClientUnaryCall;
  dispatchCommand(
    request: Record<string, unknown>,
    callback: (
      error: grpc.ServiceError | null,
      response: Record<string, unknown> | undefined,
    ) => void,
  ): grpc.ClientUnaryCall;
  subscribeEvents(request: { clientVersion: string }): grpc.ClientReadableStream<Record<string, unknown>>;
}

interface ProtoGrpcType {
  nexode: {
    hypervisor: {
      v2: {
        Hypervisor: HypervisorClientConstructor;
      };
    };
  };
}

export interface DaemonClientOptions {
  host: string;
  port: number;
  protoPath: string;
}

export class DaemonClient implements vscode.Disposable {
  private readonly snapshotEmitter = new vscode.EventEmitter<FullStateSnapshot>();
  private readonly eventEmitter = new vscode.EventEmitter<HypervisorEvent>();
  private readonly connectionEmitter = new vscode.EventEmitter<ConnectionStatus>();
  private readonly outputEmitter = new Emitter<AgentOutputLine>();
  private readonly clientConstructor: HypervisorClientConstructor;

  private host: string;
  private port: number;
  private client?: HypervisorServiceClient;
  private stream?: grpc.ClientReadableStream<Record<string, unknown>>;
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
  private reconnectAttempt = 0;
  private disposed = false;
  private generation = 0;
  private status: ConnectionStatus = { state: 'disconnected' };

  public readonly onDidReceiveSnapshot = this.snapshotEmitter.event;
  public readonly onDidChangeConnectionStatus = this.connectionEmitter.event;
  public readonly onDidReceiveAgentOutput = this.outputEmitter.event;

  public constructor(options: DaemonClientOptions) {
    this.host = options.host;
    this.port = options.port;
    this.clientConstructor = loadHypervisorClientConstructor(options.protoPath);
  }

  public subscribeEvents(callback: (event: HypervisorEvent) => void): vscode.Disposable {
    return this.eventEmitter.event(callback);
  }

  public async connect(): Promise<void> {
    this.disposed = false;
    this.clearReconnectTimer();
    await this.establishConnection(false);
  }

  public async disconnect(): Promise<void> {
    this.disposed = true;
    this.clearReconnectTimer();
    this.teardownTransport();
    this.setStatus({ state: 'disconnected' });
  }

  public dispose(): void {
    void this.disconnect();
    this.snapshotEmitter.dispose();
    this.eventEmitter.dispose();
    this.connectionEmitter.dispose();
    this.outputEmitter.dispose();
  }

  public async updateEndpoint(host: string, port: number): Promise<void> {
    if (this.host === host && this.port === port) {
      return;
    }

    this.host = host;
    this.port = port;
    this.reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
    this.reconnectAttempt = 0;
    this.clearReconnectTimer();
    this.teardownTransport();

    if (!this.disposed) {
      await this.establishConnection(false);
    }
  }

  public async getFullState(): Promise<FullStateSnapshot> {
    const client = this.requireClient();
    const response = await unaryCall<Record<string, unknown> | undefined>((callback) =>
      client.getFullState({}, callback),
    );
    return normalizeSnapshot(response);
  }

  public async dispatchCommand(command: Record<string, unknown>): Promise<CommandResponse> {
    if (this.status.state !== 'connected') {
      throw new Error('Nexode daemon is disconnected');
    }

    const client = this.requireClient();
    const response = await unaryCall<Record<string, unknown> | undefined>((callback) =>
      client.dispatchCommand(command, callback),
    );
    return normalizeCommandResponse(response);
  }

  private async establishConnection(isReconnect: boolean): Promise<void> {
    if (this.disposed) {
      return;
    }

    this.clearReconnectTimer();
    this.teardownTransport();

    const generation = ++this.generation;
    const client = this.createClient();

    try {
      await waitForReady(client);
      if (this.disposed || generation !== this.generation) {
        client.close();
        return;
      }

      this.client = client;
      const snapshot = await this.fetchFullState(client);
      this.snapshotEmitter.fire(snapshot);
      this.reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
      this.reconnectAttempt = 0;
      this.setStatus({ state: 'connected' });
      this.startStream(client, generation, isReconnect);
    } catch (error) {
      client.close();
      this.handleDisconnect(
        error instanceof Error ? error.message : 'Failed to connect to Nexode daemon',
      );
    }
  }

  private startStream(
    client: HypervisorServiceClient,
    generation: number,
    isReconnect: boolean,
  ): void {
    if (this.disposed || generation !== this.generation) {
      return;
    }

    const stream = client.subscribeEvents({
      clientVersion: CLIENT_VERSION,
    });
    this.stream = stream;

    if (isReconnect) {
      this.setStatus({ state: 'connected' });
    }

    stream.on('data', (message) => {
      if (this.disposed || generation !== this.generation) {
        return;
      }

      const event = normalizeEvent(message);
      if (event.agentOutputLine) {
        this.outputEmitter.fire(event.agentOutputLine);
      } else {
        this.eventEmitter.fire(event);
      }
    });

    stream.on('error', (error: grpc.ServiceError) => {
      if (this.disposed || generation !== this.generation) {
        return;
      }

      if (error.code === grpc.status.CANCELLED) {
        return;
      }

      this.handleDisconnect(error.message);
    });

    stream.on('end', () => {
      if (this.disposed || generation !== this.generation) {
        return;
      }

      this.handleDisconnect('Daemon event stream closed');
    });
  }

  private handleDisconnect(detail: string): void {
    if (this.disposed) {
      return;
    }

    this.teardownTransport();
    this.setStatus({ state: 'disconnected', detail });

    this.reconnectAttempt += 1;
    const delay = this.reconnectDelayMs;
    const nextRetryAt = Date.now() + delay;
    this.setStatus({
      state: 'reconnecting',
      detail,
      attempt: this.reconnectAttempt,
      nextRetryAt,
    });

    this.reconnectTimer = setTimeout(() => {
      void this.establishConnection(true);
    }, delay);

    this.reconnectDelayMs = Math.min(this.reconnectDelayMs * 2, MAX_RECONNECT_DELAY_MS);
  }

  private setStatus(status: ConnectionStatus): void {
    this.status = status;
    this.connectionEmitter.fire(status);
  }

  private createClient(): HypervisorServiceClient {
    return new this.clientConstructor(this.address(), grpc.credentials.createInsecure());
  }

  private requireClient(): HypervisorServiceClient {
    if (!this.client) {
      throw new Error('Nexode daemon client is not connected');
    }

    return this.client;
  }

  private async fetchFullState(client: HypervisorServiceClient): Promise<FullStateSnapshot> {
    const response = await unaryCall<Record<string, unknown> | undefined>((callback) =>
      client.getFullState({}, callback),
    );
    return normalizeSnapshot(response);
  }

  private teardownTransport(): void {
    if (this.stream) {
      this.stream.removeAllListeners();
      this.stream.cancel();
      this.stream = undefined;
    }

    if (this.client) {
      this.client.close();
      this.client = undefined;
    }
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
  }

  private address(): string {
    const normalizedHost = normalizeHost(this.host);
    return normalizedHost.includes(':') && !normalizedHost.startsWith('[')
      ? `[${normalizedHost}]:${this.port}`
      : `${normalizedHost}:${this.port}`;
  }
}

export function readDaemonConfiguration(): { host: string; port: number } {
  const config = vscode.workspace.getConfiguration('nexode');
  return {
    host: config.get<string>('daemonHost', 'localhost'),
    port: config.get<number>('daemonPort', 50051),
  };
}

function loadHypervisorClientConstructor(protoPath: string): HypervisorClientConstructor {
  const packageDefinition = protoLoader.loadSync(protoPath, {
    keepCase: false,
    longs: Number,
    enums: String,
    defaults: true,
    oneofs: true,
  });

  const definition = grpc.loadPackageDefinition(packageDefinition) as unknown as ProtoGrpcType;
  return definition.nexode.hypervisor.v2.Hypervisor;
}

function waitForReady(client: HypervisorServiceClient): Promise<void> {
  return new Promise((resolve, reject) => {
    client.waitForReady(Date.now() + READY_TIMEOUT_MS, (error) => {
      if (error) {
        reject(error);
        return;
      }

      resolve();
    });
  });
}

function unaryCall<TResponse>(
  invoke: (
    callback: (error: grpc.ServiceError | null, response: TResponse) => void,
  ) => grpc.ClientUnaryCall,
): Promise<TResponse> {
  return new Promise<TResponse>((resolve, reject) => {
    invoke((error, response) => {
      if (error) {
        reject(error);
        return;
      }

      resolve(response);
    });
  });
}

function normalizeHost(host: string): string {
  const trimmed = host.trim().replace(/^https?:\/\//, '').replace(/\/+$/, '');
  return trimmed || 'localhost';
}
