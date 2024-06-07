use rstd::prelude::*;
use srml_support::{dispatch::Result, StorageMap, StorageValue};
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
          Self::deposit_event(RawEvent::Approval(token_id, sender.clone(), spender.clone(), value));

          if <Allowance<T>>::exists((token_id, sender.clone(), spender.clone())) {
              <Allowance<T>>::mutate((token_id, sender, spender), |allowance| *allowance += value);
          } else {
              <Allowance<T>>::insert((token_id, sender, spender), value);
          }

          Ok(())
      }

      // the ERC20 standard transfer_from function
      // implemented in the open-zeppelin way - increase/decrease allownace
      // if approved, transfer from an account to another account without owner's signature
      pub fn transfer_from(_origin, token_id: u32, from: T::AccountId, to: T::AccountId, value: T::Balance) -> Result {
        ensure!(<Allowance<T>>::exists((token_id, from.clone(), to.clone())), "Allowance does not exist.");
        ensure!(Self::allowance((token_id, from.clone(), to.clone())) >= value, "Not enough allowance.");

        <Allowance<T>>::mutate((token_id, from.clone(), to.clone()), |allowance| *allowance -= value);
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

        Self::deposit_event(RawEvent::Transfer(token_id, from.clone(), to.clone(), value));
        
        // reduce sender's balance
        <BalanceOf<T>>::mutate((token_id, from), |from_balance| *from_balance -= value);

        // increase receiver's balance
        if <BalanceOf<T>>::exists((token_id, to.clone())) {
            <BalanceOf<T>>::mutate((token_id, to), |balance| *balance += value);
        } else {
            <BalanceOf<T>>::insert((token_id, to), value);
        }

        Ok(())
    }
}
