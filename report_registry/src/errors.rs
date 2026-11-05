use soroban_sdk::contracterror;

/// Typed errors, surfaced to clients as `Error(Contract, #n)`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not allowed to do this (wrong creator / admin).
    Unauthorized = 1,
    /// No bounty with this id.
    BountyNotFound = 2,
    /// The bounty is not open (already completed, refunded, cancelled or disputed).
    BountyNotOpen = 3,
    /// `reward_amount` must be greater than zero.
    InvalidReward = 4,
    /// `deadline` must be in the future.
    DeadlineNotInFuture = 5,
    /// `max_participants` must be at least 1.
    InvalidMaxParticipants = 6,
    /// Title or description is empty or too long.
    InvalidText = 7,
    /// The bounty creator cannot join their own bounty.
    CreatorCannotJoin = 8,
    /// This address has already joined.
    AlreadyJoined = 9,
    /// `max_participants` reached.
    BountyFull = 10,
    /// The address has not joined this bounty.
    NotParticipant = 11,
    /// The deadline has passed.
    DeadlinePassed = 12,
    /// The deadline has not passed yet.
    DeadlineNotPassed = 13,
    /// The chosen winner never submitted a solution.
    NoSubmission = 14,
    /// Refund refused: at least one solution was submitted (use a dispute).
    HasSubmissions = 15,
    /// Cancel refused: someone has already joined.
    HasParticipants = 16,
    /// The bounty is not in dispute.
    NotDisputed = 17,
    /// Bounty id counter overflowed.
    IdOverflow = 18,
    /// Contract is paused.
    ContractPaused = 19,
    /// Reentrant call detected.
    Reentrancy = 20,
}
