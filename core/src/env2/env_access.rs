// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of ink!.
//
// ink! is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// ink! is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with ink!.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
    env2::{
        call::CallData,
        property,
        GetProperty,
        SetProperty,
        Env,
        EnvTypes,
    },
};
use core::marker::PhantomData;

/// A wrapper around environments to make accessing them more efficient.
pub struct EnvAccess<E> {
    /// The wrapped environment to access.
    env: PhantomData<E>,
    /// A buffer to make environment accesses
    ///  more efficient by avoiding allocations.
    buffer: Vec<u8>,
    /// False as long as there has been no interaction between
    /// the executed contract and the environment.
    ///
    /// This flag is used to check at runtime if the environment
    /// is used correctly in respect to accessing its input.
    has_interacted: bool,
    /// True as long as the return value has not yet been set.
    ///
    /// This flag is used to check at runtime if the environment
    /// is used correctly in respect to returning its value.
    has_returned_value: bool,
}

impl<E> Default for EnvAccess<E> {
    fn default() -> Self {
        Self {
            env: Default::default(),
            buffer: Default::default(),
            has_interacted: false,
            has_returned_value: false,
        }
    }
}

impl<T> EnvTypes for EnvAccess<T>
where
    T: EnvTypes,
{
    /// The type of an address.
    type AccountId = T::AccountId;
    /// The type of balances.
    type Balance = T::Balance;
    /// The type of hash.
    type Hash = T::Hash;
    /// The type of timestamps.
    type Moment = T::Moment;
    /// The type of block number.
    type BlockNumber = T::BlockNumber;
    /// The type of a call into the runtime
    type Call = T::Call;
}

macro_rules! impl_get_property_for {
    (
        $( #[$meta:meta] )*
        fn $fn_name:ident< $prop_name:ident >() -> $ret:ty; $($tt:tt)*
    ) => {
        $( #[$meta] )*
        pub fn $fn_name(&mut self) -> $ret {
            self.assert_not_yet_returned();
            self.set_has_interacted();
            <T as GetProperty<property::$prop_name<T>>>::get_property(&mut self.buffer)
        }

        impl_get_property_for!($($tt)*);
    };
    () => {}
}

impl<T> EnvAccess<T>
where
    T: Env,
{
    /// Asserts that no value has been returned yet by the contract execution.
    pub fn assert_not_yet_returned(&self) {
        assert!(!self.has_returned_value)
    }

    /// Sets the flag for recording interaction between executed contract
    /// and environment to `true`.
    pub fn set_has_interacted(&mut self) {
        self.has_interacted = true;
    }

    impl_get_property_for! {
        /// Returns the address of the caller of the executed contract.
        fn caller<Caller>() -> T::AccountId;
        /// Returns the transferred balance for the contract execution.
        fn transferred_balance<TransferredBalance>() -> T::Balance;
        /// Returns the current price for gas.
        fn gas_price<GasPrice>() -> T::Balance;
        /// Returns the amount of gas left for the contract execution.
        fn gas_left<GasLeft>() -> T::Balance;
        /// Returns the current block time in milliseconds.
        fn now_in_ms<NowInMs>() -> T::Moment;
        /// Returns the address of the executed contract.
        fn address<Address>() -> T::AccountId;
        /// Returns the balance of the executed contract.
        fn balance<Balance>() -> T::Balance;
        /// Returns the current rent allowance for the executed contract.
        fn rent_allowance<RentAllowance>() -> T::Balance;
        /// Returns the current block number.
        fn block_number<BlockNumber>() -> T::BlockNumber;
        /// Returns the minimum balance of the executed contract.
        fn minimum_balance<MinimumBalance>() -> T::Balance;
    }

    /// Sets the rent allowance of the executed contract to the new value.
    pub fn set_rent_allowance(&mut self, new_value: T::Balance) {
        self.assert_not_yet_returned();
        self.set_has_interacted();
        <T as SetProperty<property::RentAllowance<T>>>::set_property(&mut self.buffer, &new_value)
    }

    /// Returns the input to the executed contract.
    ///
    /// # Note
    ///
    /// - The input is the 4-bytes selector followed by the arguments
    ///   of the called function in their SCALE encoded representation.
    /// - This property must be received as the first action an executed
    ///   contract to its environment and can only be queried once.
    ///   The environment access asserts this guarantee.
    pub fn input(&mut self) -> CallData {
        assert!(!self.has_interacted);
        self.assert_not_yet_returned();
        self.set_has_interacted();
        <T as GetProperty<property::Input<T>>>::get_property(&mut self.buffer)
    }

    /// Returns the value back to the caller of the executed contract.
    ///
    /// # Note
    ///
    /// The setting of this property must be the last interaction between
    /// the executed contract and its environment.
    /// The environment access asserts this guarantee.
    pub fn output<R>(&mut self, return_value: &R)
    where
        R: scale::Encode,
    {
        self.assert_not_yet_returned();
        self.set_has_interacted();
        self.has_returned_value = true;
        T::output(&mut self.buffer, &return_value);
    }
}
