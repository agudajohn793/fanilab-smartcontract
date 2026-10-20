import {
  Account,
  Keypair,
  SorobanDataBuilder,
  xdr,
} from '@stellar/stellar-sdk';
import { ContractInvoker, findExpiringPersistentKeys } from './invoker';

function ledgerKey(type: 'contractCode' | 'persistent' | 'temporary'): xdr.LedgerKey {
  if (type === 'contractCode') {
    return { type: 'contractCode' } as unknown as xdr.LedgerKey;
  }
  return {
    type: 'contractData',
    contractData: { durability: { name: type } },
  } as unknown as xdr.LedgerKey;
}

describe('Soroban TTL extension selection', () => {
  test('selects only expiring persistent contract data and code entries', () => {
    const persistent = ledgerKey('persistent');
    const temporary = ledgerKey('temporary');
    const contractCode = ledgerKey('contractCode');
    const keys = findExpiringPersistentKeys(
      [
        { key: persistent, liveUntilLedgerSeq: 580 },
        { key: temporary, liveUntilLedgerSeq: 510 },
        { key: contractCode, liveUntilLedgerSeq: 600 },
        { key: ledgerKey('persistent'), liveUntilLedgerSeq: 500 },
        { key: ledgerKey('persistent'), liveUntilLedgerSeq: 601 },
        { key: ledgerKey('persistent') },
      ],
      500,
      100
    );

    expect(keys).toEqual([persistent, contractCode]);
  });

  test('extends expiring footprint keys before submitting a signed invocation', async () => {
    const source = Keypair.random();
    const codeKey = xdr.LedgerKey.contractCode(
      new xdr.LedgerKeyContractCode({ hash: Buffer.alloc(32, 1) })
    );
    const transactionData = new SorobanDataBuilder()
      .setFootprint([codeKey], [])
      .setResources(100, 100, 100)
      .setResourceFee('0');
    let sequence = 1;
    let txHash = 0;
    const simulation = {
      _parsed: true,
      transactionData,
      minResourceFee: '100',
      latestLedger: 100,
      events: [],
      result: { auth: [] },
    };
    const server = {
      getAccount: jest.fn(async (accountId: string) => new Account(accountId, `${sequence}`)),
      simulateTransaction: jest.fn().mockResolvedValue(simulation),
      getLedgerEntries: jest.fn().mockResolvedValue({
        entries: [{ key: codeKey, liveUntilLedgerSeq: 150 }],
        latestLedger: 100,
      }),
      sendTransaction: jest.fn(async () => {
        sequence += 1;
        txHash += 1;
        return { status: 'PENDING', hash: `hash-${txHash}` };
      }),
      getTransaction: jest.fn().mockResolvedValue({ status: 'SUCCESS' }),
    };
    const invoker = new ContractInvoker(
      'CCJZ5DGASBWQXR5MPFCJXMBI333XE5U3FSJTNQU7RIKE3P5GN2K2WYD5',
      {
        sourceAccount: source.publicKey(),
        networkPassphrase: 'Test SDF Network ; September 2015',
        serverUrl: 'https://soroban-testnet.stellar.org',
        signer: source,
      }
    );
    (invoker as unknown as { server: () => never }).server = () => server as never;

    await invoker.call('read_key', []);

    expect(server.getLedgerEntries).toHaveBeenCalledWith(codeKey);
    expect(server.simulateTransaction).toHaveBeenCalledTimes(3);
    expect(server.sendTransaction).toHaveBeenCalledTimes(2);
    expect(server.sendTransaction.mock.calls[0][0].operations[0].type).toBe(
      'extendFootprintTtl'
    );
    expect(server.sendTransaction.mock.calls[1][0].operations[0].type).toBe(
      'invokeHostFunction'
    );
  });
});
