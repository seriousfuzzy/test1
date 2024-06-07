use rstd::prelude::*;
use support::{dispatch::Result, StorageMap, StorageValue, decl_storage, decl_module, decl_event, ensure};
use runtime_primitives::traits::{CheckedSub, CheckedAdd};
use {balances, system::ensure_signed};

// the module trait
// contains type definitions
pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20Token<U> {
    name: Vec<u8>,
    ticker: Vec<u8>,
    total_supply: U,
}

// public interface for this runtime module
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
      // initialize the default event for this module
      fn deposit_event<T>() = default;

      // initializes a new token
      // generates an integer token_id so that all tokens are unique
      // takes a name, ticker, total supply for the token
      // makes the initiating account the owner of the token
      // the balance of the owner is set to total supply
      fn init(_origin, name: Vec<u8>, ticker: Vec<u8>, total_supply: T::Balance) -> Result {
          let sender = ensure_signed(_origin)?;

          // checking max size for name and ticker
          // byte arrays (vecs) with no max size should be avoided
          ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
          ensure!(ticker.len() <= 32, "token ticker cannot exceed 32 bytes");

          let token_id = Self::token_id();
          <TokenId<T>>::put(token_id + 1);

          let token = Erc20Token {
              name,
              ticker,
              total_supply,
          };

          <Tokens<T>>::insert(token_id, token);
          <BalanceOf<T>>::insert((token_id, sender), total_supply);

          Ok(())
      }

      // transfer tokens from one account to another
      // origin is assumed as sender
      fn transfer(_origin, token_id: u32, to: T::AccountId, value: T::Balance) -> Result {
          let sender = ensure_signed(_origin)?;
          Self::_transfer(token_id, sender, to, value)
      }

      // approve token transfer from one account to another
      // once this is done, transfer_from can be called with corresponding values
      fn approve(_origin, token_id: u32, spender: T::AccountId, value: T::Balance) -> Result {
          let sender = ensure_signed(_origin)?;
          ensure!(<BalanceOf<T>>::exists((token_id, sender.clone())), "Account does not own this token");

          <Allowance<T>>::mutate((token_id, sender.clone(), spender.clone()), |allowance| {
              // using checked_add (safe math) to avoid overflow
              if let Some(updated_allowance) = allowance.checked_add(&value) {
                  *allowance = updated_allowance;
                }
          });

          Self::deposit_event(RawEvent::Approval(token_id, sender.clone(), spender.clone(), value));

          Ok(())
      }

      // the ERC20 standard transfer_from function
      // implemented in the open-zeppelin way - increase/decrease allownace
      // if approved, transfer from an account to another account without owner's signature
      pub fn transfer_from(_origin, token_id: u32, from: T::AccountId, to: T::AccountId, value: T::Balance) -> Result {
        ensure!(<Allowance<T>>::exists((token_id, from.clone(), to.clone())), "Allowance does not exist.");
        ensure!(Self::allowance((token_id, from.clone(), to.clone())) >= value, "Not enough allowance.");

        <Allowance<T>>::mutate((token_id, from.clone(), to.clone()), |allowance| {
              // using checked_sub (safe math) to avoid overflow
              if let Some(updated_allowance) = allowance.checked_sub(&value) {
                  *allowance = updated_allowance;
                }
          });

        Self::deposit_event(RawEvent::Approval(token_id, from.clone(), to.clone(), value));

        Self::_transfer(token_id, from, to, value)
      }
  }
}

// storage for this module
decl_storage! {
  trait Store for Module<T: Trait> as Erc20 {
      // token id nonce for storing the next token id available for token initialization
      // inspired by the AssetId in the SRML assets module
      TokenId get(token_id): u32;
      // details of the token corresponding to a token id
      Tokens get(token_details): map u32 => Erc20Token<T::Balance>;
      // balances mapping for an account and token
      BalanceOf get(balance_of): map (u32, T::AccountId) => T::Balance;
      // allowance for an account and token
      Allowance get(allowance): map (u32, T::AccountId, T::AccountId) => T::Balance;
  }
}

// events
decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId, Balance = <T as balances::Trait>::Balance {
        // event for transfer of tokens
        // tokenid, from, to, value
        Transfer(u32, AccountId, AccountId, Balance),
        // event when an approval is made
        // tokenid, owner, spender, value
        Approval(u32, AccountId, AccountId, Balance),
    }
);

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // the ERC20 standard transfer function
    // internal
    fn _transfer(
        token_id: u32,
        from: T::AccountId,
        to: T::AccountId,
        value: T::Balance,
    ) -> Result {
        ensure!(<BalanceOf<T>>::exists((token_id, from.clone())), "Account does not own this token");

        let sender_balance = Self::balance_of((token_id, from.clone()));
        ensure!(sender_balance > value, "Not enough balance.");
        let mut reduced = false;
        let mut added = false;

        // reduce sender's balance
        <BalanceOf<T>>::mutate((token_id, from.clone()), |from_balance| {
            // using checked_sub (safe math) to avoid overflow
            if let Some(updated_from_balance) = from_balance.checked_sub(&value) {
                *from_balance = updated_from_balance;
                reduced = true;
            }
        });

        // increase receiver's balance
        <BalanceOf<T>>::mutate((token_id, to.clone()), |to_balance| {
            // using checked_add (safe math) to avoid overflow
            if let Some(updated_to_balance) = to_balance.checked_add(&value) {
                if reduced == true {
                    *to_balance = updated_to_balance;
                    added = true;
                }
            }
        });

        if added == true {
            Self::deposit_event(RawEvent::Transfer(token_id, from, to, value));
            Ok(())
        } else {
            Err("Transfer failed because of overflow.")
        }
    }
}
