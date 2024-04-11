//! `ECVRF-EDWARDS25519-SHA512-TAI` suite.
//!
//! Configuration (RFC9381):
//!
//! *  suite_string = 0x03.
//!
//! *  The EC group G is the edwards25519 elliptic curve, with the finite
//!    field and curve parameters as defined in Table 1 in Section 5.1 of
//!    [RFC8032].  For this group, fLen = qLen = 32 and cofactor = 8.
//!
//! *  cLen = 16.
//!
//! *  The secret key and generation of the secret scalar and the public
//!    key are specified in Section 5.1.5 of [RFC8032].
//!
//! *  encode_to_curve_salt = PK_string.
//!
//! *  The ECVRF_nonce_generation function is as specified in
//!    Section 5.4.2.2.
//!
//! *  The int_to_string function is implemented as specified in the
//!    first paragraph of Section 5.1.2 of [RFC8032].  (This is little-
//!    endian representation.)
//!
//! *  The string_to_int function interprets the string as an integer in
//!    little-endian representation.
//!
//! *  The point_to_string function converts a point on E to an octet
//!    string according to the encoding specified in Section 5.1.2 of
//!    [RFC8032].  This implies that ptLen = fLen = 32.  (Note that
//!    certain software implementations do not introduce a separate
//!    elliptic curve point type and instead directly treat the EC point
//!    as an octet string per the above encoding.  When using such an
//!    implementation, the point_to_string function can be treated as the
//!    identity function.)
//!
//! *  The string_to_point function converts an octet string to a point
//!    on E according to the encoding specified in Section 5.1.3 of
//!    [RFC8032].  This function MUST output "INVALID" if the octet
//!    string does not decode to a point on the curve E.
//!
//! *  The hash function Hash is SHA-512 as specified in [RFC6234], with
//!    hLen = 64.
//!
//! *  The ECVRF_encode_to_curve function is as specified in
//!    Section 5.4.1.1, with interpret_hash_value_as_a_point(s) =
//!    string_to_point(s[0]...s[31]).

use crate::*;

#[derive(Copy, Clone)]
pub struct Ed25519Sha512;

suite_types!(Ed25519Sha512);

impl Suite for Ed25519Sha512 {
    const SUITE_ID: u8 = 0x03;
    const CHALLENGE_LEN: usize = 16;

    type Affine = ark_ed25519::EdwardsAffine;
    type Hash = [u8; 64];

    fn hash(data: &[u8]) -> Self::Hash {
        utils::sha512(data)
    }
}