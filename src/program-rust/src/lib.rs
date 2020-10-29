use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Sha3_256};
use solana_sdk::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, info,
    program_error::ProgramError, program_utils::next_account_info, pubkey::Pubkey,
};
use std::mem;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct Price {
    symbol: String,
    px: u64,
    last_updated: u64,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct PriceDBKeeper {
    prices: Vec<Price>,
    owner: [u8; 32],
}

impl PriceDBKeeper {
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        let x = self
            .try_to_vec()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        if x.len() != mem::size_of_val(output) {
            return Err(ProgramError::InvalidAccountData);
        }
        for i in 0..x.len() {
            output[i] = x[i];
        }
        Ok(())
    }

    pub fn new(prices: Vec<Price>, owner: [u8; 32]) -> Self {
        PriceDBKeeper { prices, owner }
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ValidatorPubkey([u8; 32]);

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum ValidatorKeeper {
    Validators(Vec<ValidatorPubkey>),
}

impl ValidatorKeeper {
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        let x = self
            .try_to_vec()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        if x.len() != mem::size_of_val(output) {
            return Err(ProgramError::InvalidAccountData);
        }
        for i in 0..x.len() {
            output[i] = x[i];
        }
        Ok(())
    }
}

/// Commands supported by the program
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum Command {
    // account 0: PriceDBKeeper account
    Init([u8; 32]),
    SetOwner([u8; 32]),
    SetPrice(Vec<Price>),
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
fn process_instruction<'a>(
    _program_id: &Pubkey, // Public key of the account the pricedb program was loaded into
    accounts: &'a [AccountInfo<'a>], // The accounts to be interacted with
    instruction_data: &[u8], // borsh encoded of Command
) -> ProgramResult {
    info!("Begin pricedb Rust program entrypoint");

    let command =
        Command::try_from_slice(instruction_data).map_err(|_| ProgramError::CustomError(999))?;
    let account_info_iter = &mut accounts.iter();

    match command {
        Command::Init(owner) => {
            info!("Init!");
            let pdbk_account = next_account_info(account_info_iter)?;

            let mut data = pdbk_account.try_borrow_mut_data()?;
            let temp = (*data).to_vec();
            match PriceDBKeeper::try_from_slice(&temp) {
                Ok(_) => Err(ProgramError::InvalidArgument),
                Err(_) => {
                    PriceDBKeeper::new(vec![], owner).serialize(&mut data)?;
                    Ok(())
                }
            }
        }
        Command::SetOwner(new_owner) => {
            info!("SetOwner!");
            let pdbk_account = next_account_info(account_info_iter)?;

            let mut data = pdbk_account.try_borrow_mut_data()?;
            let temp = (*data).to_vec();

            match PriceDBKeeper::try_from_slice(&temp) {
                Ok(PriceDBKeeper { prices, owner: _ }) => {
                    PriceDBKeeper::new(prices, new_owner).serialize(&mut data)?;
                    Ok(())
                }
                Err(_) => Err(ProgramError::InvalidArgument),
            }
        }
        Command::SetPrice(new_prices) => {
            info!("SetPrice!");
            let pdbk_account = next_account_info(account_info_iter)?;

            // Size of Price after Borsh encode is 9
            if pdbk_account.try_data_len()? < 9 {
                info!("PriceDBKeeper account data length too small for enum+u64");
                return Err(ProgramError::InvalidAccountData);
            }

            // Save new price to PriceDBKeeper account
            let mut data = pdbk_account.try_borrow_mut_data()?;
            let temp = (*data).to_vec();

            let PriceDBKeeper {
                prices: mut prices,
                owner,
            } = PriceDBKeeper::try_from_slice(&temp).unwrap();

            for price in new_prices {
                let mut replace = false;

                for rate in prices.iter_mut() {
                    if rate.symbol == price.symbol {
                        replace = true;
                        rate.px = price.px;
                        rate.last_updated = price.last_updated;
                        break;
                    }
                }
                if !replace {
                    prices.push(price)
                }
            }

            PriceDBKeeper::new(prices, owner).serialize(&mut data)?;
            Ok(())
        }
    }
}

// Sanity tests
#[cfg(test)]
mod test {
    use super::*;
    use solana_sdk::clock::Epoch;

    fn new_pubkey(id: u8) -> Pubkey {
        Pubkey::new(&vec![
            id, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ])
    }

    #[test]
    fn test_1() {
        let program_id = new_pubkey(1);
        let key = new_pubkey(2);
        let mut lamports = 0;
        let mut data = vec![0, 0, 0, 0, 0, 0, 0, 0, 0];
        let owner = new_pubkey(3);
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );

        let accounts = vec![account];

        assert_eq!(PriceDBKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),);

        process_instruction(&program_id, &accounts, &(vec![0, 10, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(
            PriceDBKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            PriceDBKeeper::LatestPrice(Price { px: 10 })
        );

        process_instruction(&program_id, &accounts, &(vec![0, 99, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(
            PriceDBKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            PriceDBKeeper::LatestPrice(Price { px: 99 })
        );
    }

    #[test]
    fn test_2() {
        let program_id = new_pubkey(1);
        let key = new_pubkey(2);
        let mut lamports = 0;
        // contain 2 validators
        let mut data =
            ValidatorKeeper::Validators(vec![ValidatorPubkey([0; 32]), ValidatorPubkey([0; 32])])
                .try_to_vec()
                .unwrap();
        let owner = new_pubkey(3);
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );

        let accounts = vec![account];

        assert_eq!(
            ValidatorKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            ValidatorKeeper::Validators(vec![ValidatorPubkey([0; 32]), ValidatorPubkey([0; 32])])
        );

        let mut pub1 = [0u8; 32];
        let mut pub2 = [0u8; 32];
        let mut tmp1 = Sha3_256::new();
        let mut tmp2 = Sha3_256::new();
        tmp1.input(b"abc");
        tmp2.input(b"def");

        pub1.copy_from_slice(tmp1.result().as_slice());
        pub2.copy_from_slice(tmp2.result().as_slice());

        process_instruction(
            &program_id,
            &accounts,
            &(Command::SetValidator(vec![ValidatorPubkey(pub1), ValidatorPubkey(pub2)]))
                .try_to_vec()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ValidatorKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            ValidatorKeeper::Validators(vec![ValidatorPubkey(pub1), ValidatorPubkey(pub2)])
        );
    }

    #[test]
    fn test_3() {
        let program_id = new_pubkey(1);
        let key1 = new_pubkey(2);
        let mut lamports1 = 0;
        // contain 2 validators
        let mut data1 =
            ValidatorKeeper::Validators(vec![ValidatorPubkey([1; 32]), ValidatorPubkey([2; 32])])
                .try_to_vec()
                .unwrap();
        let owner1 = new_pubkey(3);
        let vk_account = AccountInfo::new(
            &key1,
            false,
            true,
            &mut lamports1,
            &mut data1,
            &owner1,
            false,
            Epoch::default(),
        );

        let key2 = new_pubkey(4);
        let mut lamports2 = 0;
        let mut data2 = vec![0, 0, 0, 0, 0, 0, 0, 0, 0];
        let owner2 = new_pubkey(5);
        let pdbk_account = AccountInfo::new(
            &key2,
            false,
            true,
            &mut lamports2,
            &mut data2,
            &owner2,
            false,
            Epoch::default(),
        );

        let accounts = vec![pdbk_account, vk_account];

        assert_eq!(
            PriceDBKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            PriceDBKeeper::Unallocated(0)
        );

        assert_eq!(
            ValidatorKeeper::try_from_slice(&accounts[1].data.borrow()).unwrap(),
            ValidatorKeeper::Validators(vec![ValidatorPubkey([1; 32]), ValidatorPubkey([2; 32])])
        );

        let calldata1 = [0; 32].to_vec();
        assert_eq!(
            process_instruction(
                &program_id,
                &accounts,
                &(Command::VerifyAndSetPrice(calldata1))
                    .try_to_vec()
                    .unwrap(),
            ),
            Err(ProgramError::CustomError(998))
        );

        let mut calldata2 = [0; 32].to_vec();
        calldata2.append(&mut vec![254, 133, 13, 0, 0, 0, 0, 0]);
        assert_eq!(
            process_instruction(
                &program_id,
                &accounts,
                &(Command::VerifyAndSetPrice(calldata2))
                    .try_to_vec()
                    .unwrap(),
            ),
            Err(ProgramError::CustomError(999))
        );

        let mut calldata3 = [2; 32].to_vec();
        calldata3.append(&mut vec![254, 133, 13, 0, 0, 0, 0, 0]);
        process_instruction(
            &program_id,
            &accounts,
            &(Command::VerifyAndSetPrice(calldata3))
                .try_to_vec()
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            PriceDBKeeper::try_from_slice(&accounts[0].data.borrow()).unwrap(),
            PriceDBKeeper::LatestPrice(Price { px: 886270 })
        );
    }
}

// Required to support info! in tests
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();
