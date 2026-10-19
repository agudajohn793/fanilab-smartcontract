#![no_std]

use shared_types::{
    DriverPreferenceUpdatedEvent, EscrowContractUpdatedEvent, FaniLabError, StorageKey,
};

// Settlement contract for cross-border currency swaps
// This contract will handle currency conversions between different assets
// during escrow release for international deliveries.
//
// CRITICAL: This contract is currently a stub. Phase 3 implementation is required
// before mainnet deployment. See PRODUCTION_READINESS.md "Next Steps for Mainnet Launch."

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    EscrowContract,
    /// Per-driver preferred payout asset recorded via
    /// `set_driver_preference` (Issue #459).
    DriverPreference(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SettlementError {
    SwapNotImplemented = 1,
}

#[contract]
pub struct SettlementContract;

#[contractimpl]
impl SettlementContract {
    /// Initialize the settlement contract
    pub fn init(env: Env, admin: Address, escrow_contract: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage().instance().set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
    }

    /// Get the settlement contract administrator.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    /// Update the escrow contract address. Admin only.
    ///
    /// Emits `escrow_contract_updated` so off-chain indexers can track
    /// escrow rotations (Issue #588).
    pub fn set_escrow_contract(env: Env, admin: Address, new_escrow: Address) {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &new_escrow);
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);

        env.events().publish(
            (shared_types::events::escrow_contract_updated(&env),),
            EscrowContractUpdatedEvent {
                admin,
                new_escrow,
            },
        );
    }

    /// Get the configured escrow contract address.
    pub fn get_escrow_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
        env.storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    /// Record the asset a driver wants to be paid in.
    ///
    /// Any driver may set (or overwrite) their own preference; the caller must
    /// authenticate as the driver. Stored in persistent storage so the entry
    /// survives independently of the contract's instance entry, and both the
    /// preference and instance-entry TTLs are extended on every write (Issue #459).
    ///
    /// Emits `driver_preference_updated` so indexers can track preference
    /// changes (Issue #589).
    pub fn set_driver_preference(env: Env, driver: Address, to_token: Address) {
        driver.require_auth();
        let driver_clone = driver.clone();

        let key = DataKey::DriverPreference(driver);
        env.storage().persistent().set(&key, &to_token);
        env.storage().persistent().extend_ttl(
            &key,
            shared_types::ttl::LEDGER_TTL_THRESHOLD,
            shared_types::ttl::LEDGER_TTL_EXTEND_TO,
        );
        env.storage().instance().extend_ttl(
            shared_types::ttl::LEDGER_TTL_THRESHOLD,
            shared_types::ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (shared_types::events::driver_preference_updated(&env),),
            DriverPreferenceUpdatedEvent {
                driver: driver_clone,
                target_token: to_token,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Clear a driver's custom payout preference, routing settlements through
    /// the default native escrow token instead.
    ///
    /// Any driver may clear their own preference; the caller must authenticate
    /// as the driver (Issue #589).
    pub fn clear_driver_preference(env: Env, driver: Address) {
        driver.require_auth();
        let driver_clone = driver.clone();

        let key = DataKey::DriverPreference(driver);
        env.storage().persistent().remove(&key);

        env.events().publish(
            (shared_types::events::driver_preference_updated(&env),),
            DriverPreferenceUpdatedEvent {
                driver: driver_clone.clone(),
                target_token: driver_clone, // cleared → default routing
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Get driver's preferred asset for payment
    pub fn get_driver_preference(env: Env, driver: Address) -> Option<Address> {
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
        let key = DataKey::DriverPreference(driver);
        env.storage().persistent().get(&key)
    }

    /// Execute asset swap and transfer to driver.
    ///
    /// PRODUCTION GUARD: This function will panic if called in production before
    /// Phase 3 implementation is complete. This is intentional to prevent silent
    /// no-op swaps on mainnet with unimplemented settlement logic.
    pub fn execute_settlement_swap(
        env: Env,
        caller: Address,
        _from_token: Address,
        _to_token: Address,
        _recipient: Address,
        _amount: i128,
        _min_amount_out: i128,
    ) {
        caller.require_auth();

        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);

        let escrow_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        if caller != escrow_contract {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        // CRITICAL GUARD: Prevent execution of stub function on mainnet.
        // Phase 3 settlement logic must be implemented before production use.
        panic_with_error!(&env, SettlementError::SwapNotImplemented);
    }
}

#[cfg(test)]
mod test;
