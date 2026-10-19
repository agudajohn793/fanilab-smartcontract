#![no_std]

use shared_types::{
    is_admin, DriverPreferenceUpdatedEvent, EscrowContractUpdatedEvent, FaniLabError, StorageKey,
};

// Settlement contract for cross-border currency swaps through an admin-configured router.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, IntoVal,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    EscrowContract,
    SwapRouter,
    /// Per-driver preferred payout asset recorded via
    /// `set_driver_preference` (Issue #459).
    DriverPreference(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SettlementError {
    SwapNotImplemented = 1,
    SwapRouterNotConfigured = 2,
    InvalidSwapParameters = 3,
    SwapOutputBelowMinimum = 4,
    InsufficientSwapBalance = 5,
    SwapInputNotConsumed = 6,
    InvalidSwapDeadline = 7,
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

    /// Set the trusted swap-router contract. Admin only.
    ///
    /// The router must implement `swap_exact_tokens_for_tokens` with the
    /// argument and return-value contract used by `execute_settlement_swap`.
    pub fn set_swap_router(env: Env, admin: Address, router: Address) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::SwapRouter, &router);
        env.storage().instance().extend_ttl(
            shared_types::ttl::LEDGER_TTL_THRESHOLD,
            shared_types::ttl::LEDGER_TTL_EXTEND_TO,
        );
    }

    /// Get the configured swap-router contract.
    pub fn get_swap_router(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::SwapRouter)
    }

    /// Record the asset a driver wants to be paid in.
    ///
    /// Any driver may set (or overwrite) their own preference; the caller must
    /// authenticate as the driver. Stored in persistent storage so the entry
    /// survives independently of the contract's instance entry, and its TTL is
    /// extended on every write (Issue #459).
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

    /// Swap escrow-funded tokens through the configured router.
    pub fn execute_settlement_swap(
        env: Env,
        caller: Address,
        from_token: Address,
        to_token: Address,
        recipient: Address,
        amount: i128,
        min_amount_out: i128,
        deadline: u64,
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

        let now = env.ledger().timestamp();
        if deadline < now
            || deadline > now.saturating_add(shared_types::SETTLEMENT_SWAP_DEADLINE_WINDOW_SECONDS)
        {
            panic_with_error!(&env, SettlementError::InvalidSwapDeadline);
        }

        if amount <= 0 || min_amount_out < 0 || from_token == to_token {
            panic_with_error!(&env, SettlementError::InvalidSwapParameters);
        }

        let router: Address = env
            .storage()
            .instance()
            .get(&DataKey::SwapRouter)
            .unwrap_or_else(|| panic_with_error!(&env, SettlementError::SwapRouterNotConfigured));

        let current_contract = env.current_contract_address();
        let input_token = soroban_sdk::token::Client::new(&env, &from_token);
        let balance_before = input_token.balance(&current_contract);
        if balance_before < amount {
            panic_with_error!(&env, SettlementError::InsufficientSwapBalance);
        }
        let expiration_ledger = env
            .ledger()
            .sequence()
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&env, SettlementError::InvalidSwapParameters));

        input_token.approve(&current_contract, &router, &amount, &expiration_ledger);

        let amount_out: i128 = env.invoke_contract(
            &router,
            &soroban_sdk::Symbol::new(&env, "swap_exact_tokens_for_tokens"),
            soroban_sdk::vec![
                &env,
                current_contract.into_val(&env),
                from_token.into_val(&env),
                to_token.into_val(&env),
                recipient.into_val(&env),
                amount.into_val(&env),
                min_amount_out.into_val(&env),
                deadline.into_val(&env),
            ],
        );
        if amount_out < min_amount_out {
            panic_with_error!(&env, SettlementError::SwapOutputBelowMinimum);
        }
        if input_token.balance(&current_contract) != balance_before - amount {
            panic_with_error!(&env, SettlementError::SwapInputNotConsumed);
        }
    }
}

#[cfg(test)]
mod test;
