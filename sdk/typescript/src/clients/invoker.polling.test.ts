import { ContractInvokeOptions } from '../types/common.types';
import { ContractInvoker } from './invoker';

type TransactionResponse = {
  status: 'NOT_FOUND' | 'SUCCESS' | 'FAILED';
  returnValue?: never;
};

type PollServer = {
  getTransaction: (hash: string) => Promise<TransactionResponse>;
};

function waitForResult(
  server: PollServer,
  hash: string,
  options: ContractInvokeOptions
): Promise<unknown> {
  const invoker = Object.create(ContractInvoker.prototype) as ContractInvoker;
  const invoke = invoker as unknown as {
    waitForResult: (server: PollServer, hash: string, config: ContractInvokeOptions) => Promise<unknown>;
  };
  return invoke.waitForResult(server, hash, options);
}

describe('ContractInvoker transaction polling', () => {
  afterEach(() => {
    jest.useRealTimers();
  });

  test('uses exponential backoff capped at the configured maximum', async () => {
    jest.useFakeTimers();
    const server: PollServer = {
      getTransaction: jest
        .fn<() => Promise<TransactionResponse>>()
        .mockResolvedValueOnce({ status: 'NOT_FOUND' })
        .mockResolvedValueOnce({ status: 'NOT_FOUND' })
        .mockResolvedValueOnce({ status: 'NOT_FOUND' })
        .mockResolvedValueOnce({ status: 'SUCCESS' }),
    };

    const pending = waitForResult(server, 'hash', {
      pollingIntervalMs: 100,
      pollingMaxIntervalMs: 250,
      pollingTimeoutMs: 1000,
    });
    expect(server.getTransaction).toHaveBeenCalledTimes(1);
    await jest.advanceTimersByTimeAsync(100);
    expect(server.getTransaction).toHaveBeenCalledTimes(2);
    await jest.advanceTimersByTimeAsync(200);
    expect(server.getTransaction).toHaveBeenCalledTimes(3);
    await jest.advanceTimersByTimeAsync(250);

    await expect(pending).resolves.toBeUndefined();
    expect(server.getTransaction).toHaveBeenCalledTimes(4);
  });

  test('stops polling when the configured timeout expires', async () => {
    jest.useFakeTimers();
    const server: PollServer = {
      getTransaction: jest.fn().mockResolvedValue({ status: 'NOT_FOUND' }),
    };
    const pending = waitForResult(server, 'hash', {
      pollingIntervalMs: 100,
      pollingMaxIntervalMs: 100,
      pollingTimeoutMs: 150,
    });
    const timeoutAssertion = expect(pending).rejects.toThrow(
      'Timed out waiting for Soroban transaction: hash'
    );

    await jest.advanceTimersByTimeAsync(150);
    await timeoutAssertion;
    expect(server.getTransaction).toHaveBeenCalledTimes(2);
  });

  test('aborts an in-flight status request when its signal is aborted', async () => {
    const controller = new AbortController();
    const server: PollServer = {
      getTransaction: jest.fn(() => new Promise(() => undefined)),
    };
    const pending = waitForResult(server, 'hash', { signal: controller.signal });
    controller.abort();

    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
  });
});
