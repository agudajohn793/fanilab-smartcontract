jest.mock('../clients/invoker', () => ({
  address: (value: string) => ({ kind: 'address', value }),
  i128: (value: bigint) => ({ kind: 'i128', value }),
  ContractInvoker: class {},
}));

import { SettlementClient } from '../clients/settlement.client';

describe('SettlementClient contract bindings', () => {
  let client: SettlementClient;
  let callMock: jest.Mock;

  beforeEach(() => {
    client = new SettlementClient('CSETTLEMENTCONTRACTIDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX');
    callMock = jest.fn();
    (client as unknown as { invoker: { call: jest.Mock } }).invoker = { call: callMock };
  });

  it('initializes with both admin and escrow contract addresses', async () => {
    await client.init({ admin: 'GADMIN', escrowContract: 'GESCROW' });

    expect(callMock).toHaveBeenCalledWith('init', [
      { kind: 'address', value: 'GADMIN' },
      { kind: 'address', value: 'GESCROW' },
    ], undefined);
  });

  it('sets a driver preference using the contract argument order', async () => {
    await client.setDriverPreference({ driver: 'GDRIVER', toToken: 'GTOKEN' });

    expect(callMock).toHaveBeenCalledWith('set_driver_preference', [
      { kind: 'address', value: 'GDRIVER' },
      { kind: 'address', value: 'GTOKEN' },
    ], undefined);
  });
});
