use arrow_convert::{ArrowDeserialize, ArrowField, ArrowSerialize};
use strum_macros::FromRepr;
use tinystr::TinyAsciiStr;

use crate::repr::impl_repr_u8;

/// Attachment identifier for the C98 Uida attachment.
pub const UIDA_ATTACHMENT_ID: u8 = 98;

/// Unique report identifier attachment.
#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct UidaAttachment {
    /// Unique report identifier.
    pub uid: TinyAsciiStr<6>,
    /// Release number associated with this report.
    pub release_number: ReleaseNumber,
    /// Release status indicator.
    pub release_status: ReleaseStatusIndicator,
    /// Intermediate reject flag.
    pub intermediate_reject_flag: IntermediateRejectFlag,
}

/// Three-part release number.
#[derive(Debug, PartialEq, Clone, Copy, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct ReleaseNumber {
    /// Primary release number.
    pub primary: u8,
    /// Secondary release number.
    pub secondary: u8,
    /// Tertiary release number.
    pub tertiary: u8,
}

/// Release status indicator for the Uida attachment.
#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ReleaseStatusIndicator {
    /// Record is preliminary and not yet in an official ICOADS release.
    Preliminary = 0,
    /// Record is auxiliary and distributed outside the official release stream.
    Auxiliary = 1,
    /// Record is included in an official ICOADS release.
    Full = 2,
}

/// Intermediate reject flag for the Uida attachment.
#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum IntermediateRejectFlag {
    /// Keep in the intermediate dataset only.
    IntermediateOnly = 0,
    /// Keep in both intermediate and final datasets.
    Final = 1,
    /// Reject from both intermediate and final datasets.
    Reject = 2,
}

impl_repr_u8! {ReleaseStatusIndicator IntermediateRejectFlag}
