use starknet::ContractAddress;

#[derive(Drop, Serde, starknet::Store)]
pub struct Bounty {
    pub id: u256,
    pub creator: ContractAddress,
    pub title: felt252,
    pub description: felt252,
    pub reward_amount: u256,
    pub deadline: u64,
    pub status: u8,
    pub max_participants: u32,
    pub current_participants: u32,
}

#[starknet::interface]
pub trait IBountyContract<TContractState> {
    fn create_bounty(
        ref self: TContractState,
        title: felt252,
        description: felt252,
        reward_amount: u256,
        deadline: u64,
        max_participants: u32
    ) -> u256;
    
    fn join_bounty(ref self: TContractState, bounty_id: u256);
    fn submit_solution(ref self: TContractState, bounty_id: u256, solution_hash: felt252);
    fn select_winner(ref self: TContractState, bounty_id: u256, winner: ContractAddress);
    fn get_bounty(self: @TContractState, bounty_id: u256) -> Bounty;
    fn get_bounty_count(self: @TContractState) -> u256;
    fn is_participant(self: @TContractState, bounty_id: u256, participant: ContractAddress) -> bool;
    fn get_winner(self: @TContractState, bounty_id: u256) -> ContractAddress;
}

#[starknet::contract]
pub mod BountyContract {
    use super::{IBountyContract, Bounty};
    use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
    use starknet::storage::{Map, StorageMapReadAccess, StorageMapWriteAccess};

    #[storage]
    struct Storage {
        bounties: Map::<u256, Bounty>,
        bounty_count: u256,
        participants: Map::<(u256, ContractAddress), bool>,
        submissions: Map::<(u256, ContractAddress), felt252>,
        winners: Map::<u256, ContractAddress>,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BountyCreated: BountyCreated,
        ParticipantJoined: ParticipantJoined,
        SolutionSubmitted: SolutionSubmitted,
        WinnerSelected: WinnerSelected,
    }

    #[derive(Drop, starknet::Event)]
    struct BountyCreated {
        #[key]
        bounty_id: u256,
        creator: ContractAddress,
        reward_amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct ParticipantJoined {
        #[key]
        bounty_id: u256,
        participant: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    struct SolutionSubmitted {
        #[key]
        bounty_id: u256,
        participant: ContractAddress,
        solution_hash: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct WinnerSelected {
        #[key]
        bounty_id: u256,
        winner: ContractAddress,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.bounty_count.write(0);
    }

    #[abi(embed_v0)]
    impl BountyContractImpl of IBountyContract<ContractState> {
        fn create_bounty(
            ref self: ContractState,
            title: felt252,
            description: felt252,
            reward_amount: u256,
            deadline: u64,
            max_participants: u32
        ) -> u256 {
            let caller = get_caller_address();
            let current_time = get_block_timestamp();
            
            assert(deadline > current_time, 'Deadline must be in future');
            assert(reward_amount > 0, 'Reward must be positive');
            
            let bounty_id = self.bounty_count.read() + 1;
            self.bounty_count.write(bounty_id);
            
            let bounty = Bounty {
                id: bounty_id,
                creator: caller,
                title,
                description,
                reward_amount,
                deadline,
                status: 0,
                max_participants,
                current_participants: 0,
            };
            
            self.bounties.write(bounty_id, bounty);
            
            self.emit(BountyCreated {
                bounty_id,
                creator: caller,
                reward_amount,
            });
            
            bounty_id
        }

        fn join_bounty(ref self: ContractState, bounty_id: u256) {
            let caller = get_caller_address();
            let mut bounty = self.bounties.read(bounty_id);
            
            assert(bounty.id != 0, 'Bounty does not exist');
            assert(bounty.status == 0, 'Bounty not active');
            assert(bounty.creator != caller, 'Creator cannot join');
            assert(!self.participants.read((bounty_id, caller)), 'Already joined');
            assert(bounty.current_participants < bounty.max_participants, 'Bounty full');
            
            self.participants.write((bounty_id, caller), true);
            bounty.current_participants += 1;
            self.bounties.write(bounty_id, bounty);
            
            self.emit(ParticipantJoined { bounty_id, participant: caller });
        }

        fn submit_solution(ref self: ContractState, bounty_id: u256, solution_hash: felt252) {
            let caller = get_caller_address();
            let bounty = self.bounties.read(bounty_id);
            
            assert(bounty.id != 0, 'Bounty does not exist');
            assert(bounty.status == 0, 'Bounty not active');
            assert(self.participants.read((bounty_id, caller)), 'Not participant');
            assert(get_block_timestamp() <= bounty.deadline, 'Deadline passed');
            
            self.submissions.write((bounty_id, caller), solution_hash);
            
            self.emit(SolutionSubmitted { bounty_id, participant: caller, solution_hash });
        }

        fn select_winner(ref self: ContractState, bounty_id: u256, winner: ContractAddress) {
            let caller = get_caller_address();
            let mut bounty = self.bounties.read(bounty_id);
            
            assert(bounty.creator == caller, 'Only creator can select');
            assert(bounty.status == 0, 'Bounty not active');
            assert(self.participants.read((bounty_id, winner)), 'Not participant');
            
            bounty.status = 1;
            self.bounties.write(bounty_id, bounty);
            self.winners.write(bounty_id, winner);
            
            self.emit(WinnerSelected { bounty_id, winner });
        }

        fn get_bounty(self: @ContractState, bounty_id: u256) -> Bounty {
            self.bounties.read(bounty_id)
        }

        fn get_bounty_count(self: @ContractState) -> u256 {
            self.bounty_count.read()
        }

        fn is_participant(self: @ContractState, bounty_id: u256, participant: ContractAddress) -> bool {
            self.participants.read((bounty_id, participant))
        }

        fn get_winner(self: @ContractState, bounty_id: u256) -> ContractAddress {
            self.winners.read(bounty_id)
        }
    }
}