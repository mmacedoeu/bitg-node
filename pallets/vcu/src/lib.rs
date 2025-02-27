// This file is part of BitGreen.

// Copyright (C) 2021 BitGreen.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// SBP M1 review: missing documentation & benchmarks.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::DispatchResult;
    use frame_support::traits::UnixTime;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_support::{ensure, traits::Get};
    pub use frame_system::pallet_prelude::*;
    use frame_system::RawOrigin;
    use frame_system::{ensure_root, ensure_signed};
    use primitives::Balance;
    use sp_runtime::traits::One;
    use sp_runtime::traits::StaticLookup;
    use sp_std::convert::TryInto;

    pub type AssetGeneratingVCUContentOf<T> = AssetGeneratingVCUContent<
        u64, // seconds
        DescriptionOf<T>,
        DocumentOf<T>,
    >;

    pub type BundleAssetGeneratingVCUContentOf<T> =
        BundleAssetGeneratingVCUContent<u32, DescriptionOf<T>, BundleListOf<T>>;

    pub type DescriptionOf<T> = BoundedVec<u8, <T as Config>::MaxDescriptionLength>;
    pub type DocumentOf<T> = BoundedVec<u8, <T as Config>::MaxDocumentLength>;
    pub type BundleAssetGeneratingVCUDataOf<T> =
        BundleAssetGeneratingVCUData<<T as frame_system::Config>::AccountId>;
    pub type BundleListOf<T> =
        BoundedVec<BundleAssetGeneratingVCUDataOf<T>, <T as Config>::MaxBundleSize>;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_assets::Config<AssetId = u32, Balance = u128>
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Veera project id minimum length
        type MinPIDLength: Get<u32>;
        /// Max size of description
        type MaxDescriptionLength: Get<u32>;
        /// Max size of document length
        type MaxDocumentLength: Get<u32>;
        /// Max size of BundleAssetsGeneratingVCU
        type MaxBundleSize: Get<u32>;
        /// Unix time
        type UnixTime: UnixTime;
        // Information on runtime weights.
        //type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// AuthorizedAccountsAGV, we define authorized accounts to store/change the Assets Generating VCU (Verified Carbon Credit).
    #[pallet::storage]
    #[pallet::getter(fn get_authorized_accounts)]
    pub type AuthorizedAccountsAGV<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, DescriptionOf<T>>;

    /// AssetsGeneratingVCU (Verified Carbon Credit) should be stored on chain from the authorized accounts.
    #[pallet::storage]
    #[pallet::getter(fn asset_generating_vcu)]
    pub type AssetsGeneratingVCU<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u32,
        AssetGeneratingVCUContentOf<T>,
    >;

    /// AssetsGeneratingVCUShares The AGV shares can be minted/burned from the Authorized account up to the maximum number set in the AssetsGeneratingVCU.
    #[pallet::storage]
    #[pallet::getter(fn asset_generating_vcu_shares)]
    pub type AssetsGeneratingVCUShares<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, T::AccountId>, // agv account
            NMapKey<Blake2_128Concat, u32>,          // agv id
            NMapKey<Blake2_128Concat, T::AccountId>, // recipient
        ),
        u32,
        ValueQuery,
    >;

    /// AssetsGeneratingVCUSharesMinted the total AGV shares minted for a shareholder
    #[pallet::storage]
    #[pallet::getter(fn asset_generating_vcu_shares_minted)]
    pub type AssetsGeneratingVCUSharesMinted<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, u32, ValueQuery>;

    /// AssetsGeneratingVCUSharesMintedTotal the total AGV shares minted for a specific AGV
    #[pallet::storage]
    #[pallet::getter(fn asset_generating_vcu_shares_minted_total)]
    pub type AssetsGeneratingVCUSharesMintedTotal<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, u32, ValueQuery>;

    /// AssetsGeneratingVCUSchedule (Verified Carbon Credit) should be stored on chain from the authorized accounts.
    #[pallet::storage]
    #[pallet::getter(fn asset_generating_vcu_schedule)]
    pub type AssetsGeneratingVCUSchedule<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u32,
        AssetsGeneratingVCUScheduleContent,
    >;

    /// AssetsGeneratingVCUGenerated Minting of Scheduled VCU
    #[pallet::storage]
    #[pallet::getter(fn vcu_generated)]
    pub type AssetsGeneratingVCUGenerated<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, u64, ValueQuery>;

    /// VCUsBurnedAccounts: store the burned vcu for each account
    #[pallet::storage]
    #[pallet::getter(fn vcu_burned_account)]
    pub type VCUsBurnedAccounts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u32,
        u128,
        ValueQuery,
    >;

    /// VCUsBurned: store the burned VCU for each type of VCU token
    #[pallet::storage]
    #[pallet::getter(fn vcu_burned)]
    pub(super) type VCUsBurned<T: Config> = StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

    /// OraclesAccountMintingVCU: allow to store the account of the Oracle to mint the VCU for its AGV
    #[pallet::storage]
    #[pallet::getter(fn oracle_account_generating_vcu)]
    pub type OraclesAccountMintingVCU<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, T::AccountId>;

    /// OraclesTokenMintingVCU: allows to store the tokenid of the Oracle to mint the VCU for its AGV
    #[pallet::storage]
    #[pallet::getter(fn oracle_tokenid_generating_vcu)]
    pub type OraclesTokenMintingVCU<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, u32, ValueQuery>;

    /// BundleAssetsGeneratingVCU: a "bundle" of AGV
    #[pallet::storage]
    #[pallet::getter(fn bundle_asset_generating_vcu)]
    pub(super) type BundleAssetsGeneratingVCU<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BundleAssetGeneratingVCUContentOf<T>>;

    /// A counter of burned tokens for the signer
    #[pallet::storage]
    #[pallet::getter(fn get_burn_count)]
    pub type BurnedCounter<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u32, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Added authorized account.
        AuthorizedAccountAdded(T::AccountId),
        /// Destroyed authorized account.
        AuthorizedAccountsAGVDestroyed(T::AccountId),
        /// AssetsGeneratingVCU has been stored.
        AssetsGeneratingVCUCreated(u32),
        /// Destroyed AssetGeneratedVCU.
        AssetGeneratingVCUDestroyed(u32),
        /// Minted AssetGeneratedVCU.
        AssetsGeneratingVCUSharesMinted(T::AccountId, u32),
        /// Burned AssetGeneratedVCU.
        AssetsGeneratingVCUSharesBurned(T::AccountId, u32),
        /// Transferred AssetGeneratedVCU.
        AssetsGeneratingVCUSharesTransferred(T::AccountId),
        /// Added AssetsGeneratingVCUSchedule
        AssetsGeneratingVCUScheduleAdded(T::AccountId, u32),
        /// Destroyed AssetsGeneratingVCUSchedule
        AssetsGeneratingVCUScheduleDestroyed(T::AccountId, u32),
        /// Added AssetsGeneratingVCUGenerated.
        AssetsGeneratingVCUGenerated(T::AccountId, u32),
        /// Added VCUBurned.
        VCUsBurnedAdded(T::AccountId, u32, u32),
        /// Added OraclesAccountMintingVCU
        OraclesAccountMintingVCUAdded(T::AccountId, u32, T::AccountId),
        /// Destroyed OraclesAccountMintingVCUDestroyed
        OraclesAccountMintingVCUDestroyed(T::AccountId, u32),
        /// OracleAccountVCUMinted
        OracleAccountVCUMinted(T::AccountId, u32, T::AccountId),
        /// Added BundleAssetsGeneratingVCU
        AddedBundleAssetsGeneratingVCU(u32),
        /// Destroyed BundleAssetsGeneratingVCU
        DestroyedBundleAssetsGeneratingVCU(u32),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Invalid Description
        InvalidDescription,
        /// NumberofShares not found
        NumberofSharesNotFound,
        /// Number of share cannot be zero
        NumberofSharesCannotBeZero,
        /// Too many NumberofShares
        TooManyShares,
        /// AssetGeneratedVCU has not been found on the blockchain
        AssetGeneratingVCUNotFound,
        /// Invalid AGVId
        InvalidAGVId,
        /// Too less NumberofShares
        TooLessShares,
        /// InsufficientShares
        InsufficientShares,
        /// Got an overflow after adding
        Overflow,
        /// AssetGeneratedShares has not been found on the blockchain
        AssetGeneratedSharesNotFound,
        /// Invalid VCU Amount
        InvalidVCUAmount,
        /// AssetGeneratedVCUSchedule has not been found on the blockchain
        AssetGeneratedVCUScheduleNotFound,
        /// Asset does not exist,
        AssetDoesNotExist,
        /// AOraclesAccountMintingVCU Not Found
        OraclesAccountMintingVCUNotFound,
        /// Bundle does not exist,
        BundleDoesNotExist,
        /// The recipient has not shares minted
        RecipientSharesNotFound,
        /// Recipient Shares are less of burning shares
        RecipientSharesLessOfBurningShares,
        /// Total shares are not enough to burn the amount requested
        TotalSharesNotEnough,
        /// Invalid period in days
        InvalidPeriodDays,
        /// The minting time is not not yet arrived based on the schedule
        AssetGeneratedScheduleNotYetArrived,
        /// Token id not found in Assets Pallet
        TokenIdNotFound,
        /// The schedule is already present on chain
        AssetsGeneratingVCUScheduleAlreadyOnChain,
        /// The Oracle account is not matching the signer of the transaction
        OracleAccountNotMatchingSigner,
        /// Token for Oracle has not been found, inconsistency in stored data
        OraclesTokenMintingVCUNotFound,
        /// InsufficientVCUs
        InsufficientVCUs,
        /// Token id must have a value > 10000 because till 10000 is reserved for the Bridge pallet.
        ReservedTokenId,
        /// The AVG has not yet shares minted
        NoAVGSharesNotFound,
        /// Too many shares
        TooManyNumberofShares,
        /// AGV not found
        AssetGeneratedVCUNotFound,
        /// The account is not authorised to make the call
        NotAuthorised,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Store/update an AuthorizedAccountsAGV
        /// This function allows to store the Accounts enabled to create Assets generating VCU (AGV).
        /// `add_authorized_accounts` will accept `account_id` and `description` as parameter
        ///
        /// The dispatch origin for this call must be `Signed` by the Root.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn add_authorized_account(
            origin: OriginFor<T>,
            account_id: T::AccountId,
            description: DescriptionOf<T>,
        ) -> DispatchResult {
            // check for SUDO
            ensure_root(origin)?;
            // description is mandatory
            ensure!(!description.is_empty(), Error::<T>::InvalidDescription);
            //minimum lenght of 4 chars
            ensure!(description.len() > 4, Error::<T>::InvalidDescription);
            // add/replace the description for the account received
            AuthorizedAccountsAGV::<T>::try_mutate_exists(account_id.clone(), |desc| {
                *desc = Some(description);
                // Generate event
                Self::deposit_event(Event::AuthorizedAccountAdded(account_id));
                // Return a successful DispatchResult
                Ok(())
            })
        }

        /// Destroys an authorized account revoking its authorization
        ///
        /// The dispatch origin for this call must be `Signed` by the Root.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn destroy_authorized_account(
            origin: OriginFor<T>,
            account_id: T::AccountId,
        ) -> DispatchResult {
            // check for SUDO
            ensure_root(origin)?;
            // remove the authorized account from the state
            AuthorizedAccountsAGV::<T>::remove(account_id.clone());
            // Generate event
            Self::deposit_event(Event::AuthorizedAccountsAGVDestroyed(account_id));
            // Return a successful DispatchResult
            Ok(())
        }

        /// Create new Assets Generating VCU on chain
        ///
        /// `create_asset_generating_vcu` will accept `agv_account_id`, `agv_id` and `content` as parameter
        /// and create new Assets Generating VCU in system
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_asset_generating_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
            content: AssetGeneratingVCUContentOf<T>,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            ensure!(
                content.number_of_shares > 0,
                Error::<T>::NumberofSharesCannotBeZero
            );

            // store the asset
            AssetsGeneratingVCU::<T>::try_mutate_exists(agv_account_id, agv_id, |desc| {
                *desc = Some(content);
                // Generate event
                Self::deposit_event(Event::AssetsGeneratingVCUCreated(agv_id));
                // Return a successful DispatchResult
                Ok(())
            })
        }

        /// Destroy Assets Generating VCU from storage.
        ///
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn destroy_asset_generating_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset generated VCU exists or not
            ensure!(
                AssetsGeneratingVCU::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetGeneratingVCUNotFound
            );
            // TODO check for VCU already generated to avoid orphans or leave the decision to the administrator?
            // renove the assets generating VCU
            AssetsGeneratingVCU::<T>::remove(agv_account_id, agv_id);
            // Generate event
            Self::deposit_event(Event::AssetGeneratingVCUDestroyed(agv_id));
            // Return a successful DispatchResult
            Ok(())
        }

        /// The AGV shares can be minted from the Authorized account up to the maximum number set in the AssetsGeneratingVCU.
        ///
        /// ex: agvaccout: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        // TODO : Rename as mint_into
        pub fn mint_shares_asset_generating_vcu(
            origin: OriginFor<T>,
            recipient: T::AccountId,
            agv_account: T::AccountId,
            agv_id: u32,
            number_of_shares: u32,
        ) -> DispatchResult {
            // checking for SUDO or authorized account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset generating VCU (AGV) exists or not
            // read info about the AGV
            let content: AssetGeneratingVCUContentOf<T> =
                AssetsGeneratingVCU::<T>::get(&agv_account, &agv_id)
                    .ok_or(Error::<T>::AssetGeneratingVCUNotFound)?;

            // increase the total shares minted for the recipient/shareholder
            AssetsGeneratingVCUSharesMinted::<T>::try_mutate(
                &agv_account,
                &agv_id,
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_add(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    ensure!(
                        total_sh <= content.number_of_shares,
                        Error::<T>::TooManyShares
                    );
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // increase the total shares minted per AGV
            AssetsGeneratingVCUSharesMintedTotal::<T>::try_mutate(
                &agv_account,
                &agv_id,
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_add(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    ensure!(
                        total_sh <= content.number_of_shares,
                        Error::<T>::TooManyShares
                    );
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // increase the shares minted for the recipient
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &recipient),
                |share| -> DispatchResult {
                    let total_sha = share
                        .checked_add(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    *share = total_sha;
                    Ok(())
                },
            )?;

            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUSharesMinted(agv_account, agv_id));
            // Return a successful DispatchResult
            Ok(())
        }

        /// The AGV shares can be burned from the Authorized account in the AssetsGeneratingVCU.
        ///
        /// ex: agv_id: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        // TODO : Rename as burn_from
        pub fn burn_shares_asset_generating_vcu(
            origin: OriginFor<T>,
            recipient: T::AccountId,
            agv_account: T::AccountId,
            agv_id: u32,
            number_of_shares: u32,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset generated VCU exists or not
            ensure!(
                AssetsGeneratingVCU::<T>::contains_key(&agv_account, &agv_id),
                Error::<T>::AssetGeneratingVCUNotFound
            );
            // check for previously minted shares for the recipient
            ensure!(
                AssetsGeneratingVCUShares::<T>::contains_key((&agv_account, &agv_id, &recipient),),
                Error::<T>::RecipientSharesNotFound
            );
            // check  the number of burnable shares for the recipient
            let currentshares = AssetsGeneratingVCUShares::<T>::get((
                agv_account.clone(),
                agv_id,
                recipient.clone(),
            ));
            ensure!(
                currentshares >= number_of_shares,
                Error::<T>::RecipientSharesLessOfBurningShares
            );
            // check the number of burnable shares in total
            ensure!(
                AssetsGeneratingVCUSharesMinted::<T>::contains_key(&agv_account, &agv_id),
                Error::<T>::TotalSharesNotEnough
            );
            let totalcurrentshares =
                AssetsGeneratingVCUSharesMinted::<T>::get(&agv_account, &agv_id);
            ensure!(
                totalcurrentshares >= number_of_shares,
                Error::<T>::TotalSharesNotEnough
            );
            // decrease total shares minted
            AssetsGeneratingVCUSharesMinted::<T>::try_mutate(
                &agv_account,
                &agv_id,
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_sub(number_of_shares)
                        .ok_or(Error::<T>::InsufficientShares)?;
                    // TODO : Is this check needed??
                    ensure!(total_sh > 0, Error::<T>::TooLessShares);
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // decrease shares minted for the recipient account
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &recipient),
                |share| -> DispatchResult {
                    let total_sha = share
                        .checked_sub(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    *share = total_sha;
                    Ok(())
                },
            )?;
            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUSharesBurned(agv_account, agv_id));
            // Return a successful DispatchResult
            Ok(())
        }
        /// The owner can transfer its own shares to a recipient
        ///
        /// ex: agv_id: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn transfer_shares_asset_generating_vcu(
            origin: OriginFor<T>,
            recipient: T::AccountId,
            agv_account: T::AccountId,
            agv_id: u32,
            number_of_shares: u32,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // check that the shares are present
            ensure!(
                AssetsGeneratingVCUShares::<T>::contains_key((&agv_account, &agv_id, &sender)),
                Error::<T>::AssetGeneratedSharesNotFound
            );
            // get the shares available
            let sender_shares =
                AssetsGeneratingVCUShares::<T>::get((&agv_account, &agv_id, &sender));
            // check whether shares are enough for the transfer
            ensure!(
                number_of_shares <= sender_shares,
                Error::<T>::NumberofSharesNotFound
            );
            // decrease the shares for the sender
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &sender),
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_sub(number_of_shares)
                        .ok_or(Error::<T>::TooLessShares)?;
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // increase the shares for the recipient for the same amount
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &recipient),
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_add(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUSharesTransferred(recipient));
            // Return a successful DispatchResult
            Ok(())
        }
        /// The administrator can force a transfer of shares from a sender to a recipient
        ///
        /// ex: agv_id: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        // TODO : Rename with proper camel case
        pub fn forcetransfer_shares_asset_generating_vcu(
            origin: OriginFor<T>,
            sender: T::AccountId,
            recipient: T::AccountId,
            agv_account: T::AccountId,
            agv_id: u32,
            number_of_shares: u32,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check that the shares are present
            ensure!(
                AssetsGeneratingVCUShares::<T>::contains_key((&agv_account, &agv_id, &sender)),
                Error::<T>::AssetGeneratedSharesNotFound
            );
            // get the shares available
            let sender_shares =
                AssetsGeneratingVCUShares::<T>::get((&agv_account, &agv_id, &sender));
            // check whether shares are enough for the transfer
            ensure!(
                number_of_shares <= sender_shares,
                Error::<T>::NumberofSharesNotFound
            );
            // decrease the shares for the sender
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &sender),
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_sub(number_of_shares)
                        .ok_or(Error::<T>::TooLessShares)?;
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // increase the shares for the recipient for the same amount
            AssetsGeneratingVCUShares::<T>::try_mutate(
                (&agv_account, &agv_id, &recipient),
                |share| -> DispatchResult {
                    let total_sh = share
                        .checked_add(number_of_shares)
                        .ok_or(Error::<T>::Overflow)?;
                    *share = total_sh;
                    Ok(())
                },
            )?;
            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUSharesTransferred(recipient));
            // Return a successful DispatchResult
            Ok(())
        }

        /// To store asset generating vcu schedule
        ///
        /// ex: agv_account: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_asset_generating_vcu_schedule(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
            period_days: u64,
            amount_vcu: Balance,
            token_id: u32,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset generating VCU exists or not
            ensure!(
                AssetsGeneratingVCU::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetGeneratingVCUNotFound
            );
            // check for VCU amount > 0
            ensure!(amount_vcu > 0, Error::<T>::InvalidVCUAmount);
            // check for days >0
            ensure!(period_days > 0, Error::<T>::InvalidPeriodDays);
            // check the schedule is not alreayd on chain
            ensure!(
                !AssetsGeneratingVCUSchedule::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetsGeneratingVCUScheduleAlreadyOnChain
            );

            // check the token id is present on chain
            ensure!(
                pallet_assets::Pallet::<T>::maybe_total_supply(token_id).is_some(),
                Error::<T>::TokenIdNotFound
            );

            // check the token id > 10000 (because under 10000 reserver for the bridge)
            ensure!(token_id >= 10000, Error::<T>::ReservedTokenId);

            // store the schedule
            AssetsGeneratingVCUSchedule::<T>::insert(
                &agv_account_id,
                &agv_id,
                AssetsGeneratingVCUScheduleContent {
                    period_days,
                    amount_vcu,
                    token_id,
                },
            );
            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUScheduleAdded(
                agv_account_id,
                agv_id,
            ));
            // Return a successful DispatchResult
            Ok(())
        }

        /// To destroy asset generating vcu schedule
        ///
        /// ex: agv_id: 5Hdr4DQufkxmhFcymTR71jqYtTnfkfG5jTs6p6MSnsAcy5ui-1
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn destroy_asset_generating_vcu_schedule(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset generated VCU exists or not
            ensure!(
                AssetsGeneratingVCUSchedule::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetGeneratedVCUScheduleNotFound
            );
            // remove the schedule
            AssetsGeneratingVCUSchedule::<T>::remove(&agv_account_id, &agv_id);
            // Generate event
            Self::deposit_event(Event::AssetsGeneratingVCUScheduleDestroyed(
                agv_account_id,
                agv_id,
            ));
            // Return a successful DispatchResult
            Ok(())
        }

        /// This function allows the minting of the VCU periodically. The function must be accessible only from SUDO account or one of the accounts stored in AuthorizedAccountsAGV.
        ///
        /// This function checks if it’s time to mint new VCU based on the schedule and the previous generated VCU stored in AssetsGeneratingVCUGenerated or
        /// if it’s time to generate new VCU, it mints the scheduled “Assets” (see Assets pallets), and stores in AssetsGeneratingVCUGenerated  a json structure with the following fields:
        /// ```json
        /// {
        /// “timestamp”: u32  (epoch time in seconds)
        /// “amountvcu”: i32,
        /// }
        /// ```
        /// The function must deny further minting once is done till the new schedule is expired.
        /// For example with a schedule every year, the minting will be executed only one time every 365 days.
        ///
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        /// the first minting can be done anytime, the  following minting not before the scheduled time
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn mint_scheduled_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
        ) -> DispatchResultWithPostInfo {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check for AGV
            ensure!(
                AssetsGeneratingVCUSchedule::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetGeneratedVCUScheduleNotFound
            );
            let content: AssetsGeneratingVCUScheduleContent =
                AssetsGeneratingVCUSchedule::<T>::get(agv_account_id.clone(), &agv_id).unwrap();

            let mut timestamp: u64 = 0;
            let now: u64 = T::UnixTime::now().as_secs();
            // check for the last minting done
            if AssetsGeneratingVCUGenerated::<T>::contains_key(&agv_account_id, &agv_id) {
                timestamp = AssetsGeneratingVCUGenerated::<T>::get(&agv_account_id, &agv_id);
            }

            // TODO : Replace time calculation with blocks calculation
            let elapse: u64 = content.period_days * 24 * 60;
            ensure!(
                timestamp + now <= elapse,
                Error::<T>::AssetGeneratedScheduleNotYetArrived
            );
            // create token if it does not exists
            ensure!(content.token_id >= 10000, Error::<T>::ReservedTokenId);
            if let None = pallet_assets::Pallet::<T>::maybe_total_supply(content.token_id) {
                pallet_assets::Pallet::<T>::force_create(
                    RawOrigin::Root.into(),
                    content.token_id,
                    T::Lookup::unlookup(agv_account_id.clone()),
                    Default::default(),
                    One::one(),
                )?;
            }
            // check for existing shares
            ensure!(
                AssetsGeneratingVCUSharesMintedTotal::<T>::contains_key(
                    agv_account_id.clone(),
                    agv_id.clone()
                ),
                Error::<T>::NoAVGSharesNotFound
            );
            // read totals shares minted for the AGV
            let totalshares: u128 = AssetsGeneratingVCUSharesMintedTotal::<T>::get(
                agv_account_id.clone(),
                agv_id.clone(),
            )
            .into();
            // set the key of search
            // TODO : Replace this with BTreeMap, this is unbounded!!
            let shareholdersc =
                AssetsGeneratingVCUShares::<T>::iter_prefix((agv_account_id.clone(), agv_id));
            let nshareholders = shareholdersc.count();
            // iter for the available shareholders
            let shareholders =
                AssetsGeneratingVCUShares::<T>::iter_prefix((agv_account_id.clone(), agv_id));
            let mut vcuminted: u128 = 0;
            let mut nshareholdersprocessed: usize = 0;
            for numsh in shareholders {
                let shareholder = numsh.0;
                let numshares: u128 = numsh.1.into();
                //compute VCU for the shareholder
                let mut nvcu = content.amount_vcu / totalshares * numshares;
                // increase counter shareholders processed
                nshareholdersprocessed = nshareholdersprocessed + 1;
                // manage overflow for rounding
                if nshareholdersprocessed == nshareholders && vcuminted + nvcu > content.amount_vcu
                {
                    nvcu = content.amount_vcu - vcuminted;
                }
                // manage underflow for rounding
                if nshareholdersprocessed == nshareholders && vcuminted + nvcu < content.amount_vcu
                {
                    nvcu = content.amount_vcu - vcuminted;
                }
                //mint the vcu in proportion to the shares owned
                // TODO : Remove tight coupling and use traits
                pallet_assets::Pallet::<T>::mint(
                    RawOrigin::Signed(agv_account_id.clone()).into(),
                    content.token_id,
                    T::Lookup::unlookup(shareholder.clone()),
                    nvcu,
                )?;
                // increase counter minted
                vcuminted = vcuminted + nvcu;
            }
            // mint the assets
            // store the last minting time in AssetsGeneratingVCUGenerated
            if AssetsGeneratingVCUGenerated::<T>::contains_key(&agv_account_id, &agv_id) {
                AssetsGeneratingVCUGenerated::<T>::take(&agv_account_id, &agv_id);
            }
            AssetsGeneratingVCUGenerated::<T>::insert(&agv_account_id, &agv_id, now);
            // generate event
            Self::deposit_event(Event::AssetsGeneratingVCUGenerated(agv_account_id, agv_id));
            // return
            Ok(().into())
        }

        /// The owner of the “VCUs”  can decide anytime to “retire”, basically burning them.
        ///
        /// The dispatch origin for this call must be `Signed` from the owner of the VCU
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn retire_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            // check for a signed transaction
            let sender = ensure_signed(origin)?;

            let content: AssetsGeneratingVCUScheduleContent =
                AssetsGeneratingVCUSchedule::<T>::get(agv_account_id.clone(), &agv_id)
                    .ok_or(Error::<T>::AssetGeneratedVCUScheduleNotFound)?;

            // check for enough balance
            let amount_vcu = pallet_assets::Pallet::<T>::balance(content.token_id, sender.clone());
            ensure!(amount_vcu >= amount, Error::<T>::InsufficientVCUs);

            // burn the tokens on assets pallet for the requested amount
            pallet_assets::Pallet::<T>::burn(
                RawOrigin::Signed(agv_account_id.clone()).into(),
                content.token_id,
                T::Lookup::unlookup(sender.clone()),
                amount,
            )?;
            // increase the counter of burned VCU for the signer of th transaction
            BurnedCounter::<T>::try_mutate(
                &sender,
                &content.token_id,
                |count| -> DispatchResult {
                    *count += 1;
                    Ok(())
                },
            )?;
            //increase burned VCU for the AGV
            VCUsBurnedAccounts::<T>::try_mutate(
                &agv_account_id,
                &agv_id,
                |vcu| -> DispatchResult {
                    let total_vcu = vcu.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                    *vcu = total_vcu;
                    Ok(())
                },
            )?;
            // increase global counter burned VCU
            VCUsBurned::<T>::try_mutate(&content.token_id, |vcu| -> DispatchResult {
                let total_vcu = vcu.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                *vcu = total_vcu;
                Ok(())
            })?;
            // Generate event
            Self::deposit_event(Event::VCUsBurnedAdded(
                agv_account_id,
                agv_id,
                content.token_id,
            ));
            // Return a successful DispatchResult
            Ok(().into())
        }

        /// The VCUs may be generated from Oracle collecting data from off-chain. For example a Solar Panel field may have an Oracle collecting the
        /// output power and generating the VCUs periodically on Chain. We have allowed the account of the Oracle to mint the VCU for his AGV.
        ///
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_oracle_account_minting_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
            oracle_account_id: T::AccountId,
            token_id: u32,
        ) -> DispatchResult {
            // check for SUDO or administrator accounts
            Self::ensure_root_or_authorized_account(origin)?;

            // check if the AGV exists or not
            ensure!(
                AssetsGeneratingVCU::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::AssetGeneratingVCUNotFound
            );
            // check token id >10000
            ensure!(token_id >= 10000, Error::<T>::ReservedTokenId);
            // store the token if assigned for the Oracle
            if OraclesTokenMintingVCU::<T>::contains_key(agv_account_id.clone(), agv_id.clone()) {
                OraclesTokenMintingVCU::<T>::take(agv_account_id.clone(), agv_id.clone());
            }
            OraclesTokenMintingVCU::<T>::insert(
                agv_account_id.clone(),
                agv_id.clone(),
                token_id.clone(),
            );
            //store the oracle or replace if already present, we allow only one oracle for each AGV
            OraclesAccountMintingVCU::<T>::try_mutate_exists(
                agv_account_id.clone(),
                agv_id,
                |oracle| {
                    *oracle = Some(oracle_account_id.clone());
                    // Generate event
                    Self::deposit_event(Event::OraclesAccountMintingVCUAdded(
                        agv_account_id,
                        agv_id,
                        oracle_account_id,
                    ));
                    // Return a successful DispatchResult
                    Ok(())
                },
            )
        }

        /// Removes Oracles Generating VCU from storage.
        ///
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn destroy_oracle_account_minting_vcu(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
        ) -> DispatchResult {
            //store the oracle or replace if already present, we allow only one oracle for each AGV
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check for Oracle presence on chain
            ensure!(
                OraclesAccountMintingVCU::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::OraclesAccountMintingVCUNotFound
            );
            // remove the Oracle Account
            OraclesAccountMintingVCU::<T>::remove(agv_account_id.clone(), &agv_id);
            // remove the Oracle Token Id
            OraclesTokenMintingVCU::<T>::remove(agv_account_id.clone(), &agv_id);
            // Generate event
            Self::deposit_event(Event::OraclesAccountMintingVCUDestroyed(
                agv_account_id,
                agv_id,
            ));
            // Return a successful DispatchResult
            Ok(())
        }

        /// Mints Oracles Generating VCUs
        ///
        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn mint_vcu_from_oracle(
            origin: OriginFor<T>,
            agv_account_id: T::AccountId,
            agv_id: u32,
            amount_vcu: Balance,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // check for Oracle presence on chain
            // check for matching signer with Oracle Account
            let oracle_account: T::AccountId =
                OraclesAccountMintingVCU::<T>::get(&agv_account_id, &agv_id)
                    .ok_or(Error::<T>::OraclesAccountMintingVCUNotFound)?;
            ensure!(
                oracle_account == sender,
                Error::<T>::OracleAccountNotMatchingSigner
            );
            // check for Token id in Oracle configuration
            ensure!(
                OraclesTokenMintingVCU::<T>::contains_key(&agv_account_id, &agv_id),
                Error::<T>::OraclesTokenMintingVCUNotFound
            );
            // get the token id
            let token_id = OraclesTokenMintingVCU::<T>::get(&agv_account_id, &agv_id);
            // create token if it does not exist yet
            if let None = pallet_assets::Pallet::<T>::maybe_total_supply(token_id) {
                pallet_assets::Pallet::<T>::force_create(
                    RawOrigin::Root.into(),
                    token_id,
                    T::Lookup::unlookup(oracle_account.clone()),
                    false,
                    One::one(),
                )?;
            }
            // check for existing shares
            ensure!(
                AssetsGeneratingVCUSharesMintedTotal::<T>::contains_key(
                    agv_account_id.clone(),
                    agv_id.clone()
                ),
                Error::<T>::NoAVGSharesNotFound
            );
            // read totals shares minted for the AGV
            let totalshares: u128 = AssetsGeneratingVCUSharesMintedTotal::<T>::get(
                agv_account_id.clone(),
                agv_id.clone(),
            )
            .into();
            // set the key of search
            let shareholdersc =
                AssetsGeneratingVCUShares::<T>::iter_prefix((agv_account_id.clone(), agv_id));
            let nshareholders = shareholdersc.count();
            // iter for the available shareholders
            let shareholders =
                AssetsGeneratingVCUShares::<T>::iter_prefix((agv_account_id.clone(), agv_id));
            let mut vcuminted: u128 = 0;
            let mut nshareholdersprocessed: usize = 0;
            for numsh in shareholders {
                let shareholder = numsh.0;
                let numshares: u128 = numsh.1.into();
                //compute VCU for the shareholder
                let mut nvcu = amount_vcu / totalshares * numshares;
                // increase counter shareholders processed
                nshareholdersprocessed = nshareholdersprocessed + 1;
                // manage overflow for rounding
                if nshareholdersprocessed == nshareholders && vcuminted + nvcu > amount_vcu {
                    nvcu = amount_vcu - vcuminted;
                }
                // manage underflow for rounding
                if nshareholdersprocessed == nshareholders && vcuminted + nvcu < amount_vcu {
                    nvcu = amount_vcu - vcuminted;
                }
                //mint the vcu in proportion to the shares owned
                pallet_assets::Pallet::<T>::mint(
                    RawOrigin::Signed(agv_account_id.clone()).into(),
                    token_id,
                    T::Lookup::unlookup(shareholder.clone()),
                    nvcu,
                )?;
                // increase counter minted
                vcuminted = vcuminted + nvcu;
            }
            // here the total vcu minted should be exactly the amount received as parameter.
            // generate event
            Self::deposit_event(Event::OracleAccountVCUMinted(
                agv_account_id,
                agv_id,
                oracle_account,
            ));
            Ok(().into())
        }

        /// The dispatch origin for this call must be `Signed` either by the Root or authorized account.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_bundle_agv(
            origin: OriginFor<T>,
            bundle_id: u32,
            info: BundleAssetGeneratingVCUContentOf<T>,
        ) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check whether asset exists or not
            ensure!(
                pallet_assets::Pallet::<T>::maybe_total_supply(info.asset_id).is_some(),
                Error::<T>::AssetDoesNotExist
            );

            for agv in info.bundle.clone() {
                // check whether asset generated VCU exists or not
                ensure!(
                    AssetsGeneratingVCU::<T>::contains_key(&agv.account_id, &agv.id),
                    Error::<T>::AssetGeneratingVCUNotFound
                );
            }

            BundleAssetsGeneratingVCU::<T>::insert(&bundle_id, &info);
            Self::deposit_event(Event::AddedBundleAssetsGeneratingVCU(bundle_id));

            Ok(())
        }

        /// Destroys an AGV bundle from storage.
        ///
        /// The dispatch origin for this call must be `Signed` by the Root.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn destroy_bundle_agv(origin: OriginFor<T>, bundle_id: u32) -> DispatchResult {
            // check for Sudo or other admnistrator account
            Self::ensure_root_or_authorized_account(origin)?;

            // check if the bundle is on chain
            ensure!(
                BundleAssetsGeneratingVCU::<T>::contains_key(&bundle_id),
                Error::<T>::BundleDoesNotExist
            );
            // remove the bundle from the chain
            BundleAssetsGeneratingVCU::<T>::remove(bundle_id);
            // Generate event
            Self::deposit_event(Event::DestroyedBundleAssetsGeneratingVCU(bundle_id));
            // Return a successful DispatchResult
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        // Ensure the origin is Sudo key or an authorised account
        pub fn ensure_root_or_authorized_account(origin: OriginFor<T>) -> DispatchResult {
            match ensure_root(origin.clone()) {
                Ok(()) => Ok(()),
                Err(e) => ensure_signed(origin).and_then(|o: T::AccountId| {
                    if AuthorizedAccountsAGV::<T>::contains_key(&o) {
                        Ok(())
                    } else {
                        Err(e)
                    }
                }),
            }
            .map_err(|_| Error::<T>::NotAuthorised.into())
        }
    }
}
