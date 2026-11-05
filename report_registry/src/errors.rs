use soroban_sdk::contracterror;

/// Typed errors, surfaced to clients as `Error(Contract, #n)`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not allowed to do this.
    Unauthorized = 1,
    /// No report with this id.
    ReportNotFound = 2,
    /// This exact report hash has already been published.
    AlreadyPublished = 3,
    /// A text field is empty or too long.
    InvalidText = 4,
    /// Page `limit` is 0 or above `MAX_PAGE_SIZE`.
    InvalidPageSize = 5,
    /// The report is already revoked.
    AlreadyRevoked = 6,
    /// Report id counter overflowed.
    IdOverflow = 7,
    /// Publishing is paused.
    ContractPaused = 8,
}
