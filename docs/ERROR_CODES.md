# Error Code Reference

Canonical, single-source-of-truth reference for every `#[contracterror]` enum in the
FaniLab workspace. `docs/API.md`'s "Error Codes by Contract" section links here rather
than maintaining a second copy — update this file when an error variant is added,
removed, or renumbered, and both docs stay accurate.

## Why this file exists

A raw Soroban error only carries a numeric code: `Error(Contract, #N)`. That number's
meaning depends entirely on *which contract* raised it — each contract defines its own
`#[contracterror]` enum starting from `1`, so the same code means different things in
different contracts. An integrator handling errors from a multi-contract call chain
(e.g. `delivery_contract` invoking `escrow_contract` invoking `identity_reputation_contract`)
needs to know which contract's enum to look the code up against. This file lists every
enum in the workspace so any `(contract, code)` pair can be resolved in one place.

Two contracts — `dispute_resolution_contract` and `identity_reputation_contract` —
define no local error enum and raise only the shared `FaniLabError` below.

## `FaniLabError` — shared across all six contracts (`shared_types`)

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `Unauthorized` | Caller is not authorized to perform the requested action. |
| 2 | `AlreadyInitialized` | Contract or protocol state has already been initialized. |
| 3 | `NotInitialized` | Contract or protocol state has not been initialized yet. |
| 4 | `DeliveryNotFound` | Delivery record or related escrow entry could not be found. |
| 5 | `InvalidState` | Requested operation is invalid for the current protocol state. |
| 6 | `InsufficientFunds` | Contract balance is too low to complete the requested transfer. |
| 8 | `DuplicateDelivery` | Delivery identifier already exists in protocol storage. |
| 9 | `ProviderNotFound` | Provider or driver record could not be found. |
| 11 | `ProtocolPaused` | Protocol is paused and fund movements are halted. |
| 12 | `LimitExceeded` | Requested operation would exceed a fixed capacity/growth limit (e.g. `dispute_resolution_contract::add_evidence_hash` once the calling party has already submitted its per-party maximum of 20 evidence hashes for that dispute — the cap is enforced per submitting party, so at most `3 × 20` hashes accumulate across the three delivery parties). |
| 13 | `TimelockNotElapsed` | A requested timelocked operation was attempted before its delay elapsed. |

Note: codes `7` and `10` are intentionally unused gaps left by prior variant removals —
they are not reserved for anything and should not be inferred to mean "no error."

## `EscrowError` — `escrow_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 5 | `InvalidFee` | Requested platform fee exceeds the configured maximum. |
| 6 | `InvalidToken` | Token does not match the protocol-configured token. |
| 7 | `InvalidAmount` | Escrow amount is not positive. |
| 8 | `NoPendingSettlementChange` | `confirm_settlement_contract` called with no proposal pending. |
| 10 | `InvalidDriver` | Driver address matches the sender or recipient, which is not permitted for escrow creation. |
| 11 | `InvalidParties` | Sender and recipient are the same address. |
| 12 | `BatchTooLarge` | `create_escrows_batch` list exceeds `MAX_BATCH_SIZE` (100). |

Shared failures such as `InvalidState`, `DeliveryNotFound`, `InsufficientFunds`,
`DuplicateDelivery`, `TimelockNotElapsed`, `Unauthorized`, `NotInitialized`, and
`ProtocolPaused` use `FaniLabError`.

## `DeliveryError` — `delivery_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 2 | `InvalidMetadata` | Delivery metadata fails validation (e.g. location or weight limits). |
| 3 | `BatchTooLarge` | `create_deliveries_batch` list exceeds `MAX_BATCH_SIZE` (100). |
| 4 | `InvalidDriver` | Driver address matches the delivery's sender or recipient. |
| 5 | `InvalidParties` | Sender and recipient are the same address. |
| 6 | `EscrowNotLocked` | The escrow is absent or not locked when funded escrow is required. |

Shared failures such as `InvalidState`, `Unauthorized`, `DeliveryNotFound`, and
`NotInitialized` use `FaniLabError`.

## `FleetError` — `fleet_management_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 4 | `FleetNotFound` | No fleet exists for the given fleet ID. |
| 5 | `DriverAlreadyInvited` | Driver already has a pending invite to this fleet. |
| 6 | `InviteNotFound` | No pending invite exists for this driver/fleet pair. |
| 7 | `DriverAlreadyActive` | Driver is already an active member of this fleet. |
| 8 | `NoPendingTreasuryChange` | `confirm_fleet_treasury_update` called with no proposal pending. |
| 10 | `FleetInactive` | Requested operation is invalid because the fleet has been deactivated. |
| 11 | `InvalidConfiguration` | Signer configuration contains an invalid threshold, or a fleet was reactivated while already active. |
| 12 | `InternalStorageError` | Roster compaction read a storage slot that should have existed but was absent. |
| 13 | `DriverNotRegistered` | `accept_fleet_invite` was called by an address with no `DriverProfile` in the configured identity/reputation contract (Issue #451). |
| 14 | `RosterFull` | The fleet already holds `MAX_ROSTER_SIZE` active drivers. |

Shared failures such as `AlreadyInitialized`, `NotInitialized`, `Unauthorized`, and
`TimelockNotElapsed` use `FaniLabError`.

## `SettlementError` — `settlement_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `SwapNotImplemented` | Legacy error; settlement swaps are routed through the configured swap router. |
| 2 | `SwapRouterNotConfigured` | No swap router has been configured. |
| 3 | `InvalidSwapParameters` | Swap amount, token pair, or minimum output is invalid. |
| 4 | `SwapOutputBelowMinimum` | Router output is below the requested minimum. |
| 5 | `InsufficientSwapBalance` | Settlement does not hold the requested input amount. |
| 6 | `SwapInputNotConsumed` | Router did not consume the exact approved input amount. |
| 7 | `InvalidSwapDeadline` | Swap deadline is expired or exceeds the 300-second window. |

## Contracts with no local error enum

`dispute_resolution_contract` and `identity_reputation_contract` raise `FaniLabError`
exclusively — look up any error code they return in the shared table above.

## Interpreting a raw error

Given `Error(Contract, #N)`, identify which contract's invocation raised it. Resolve
contract-specific errors against that contract's table; shared `FaniLabError` errors use
the same code mapping in every contract.
