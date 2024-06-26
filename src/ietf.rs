//! EC-VRF as specified by [RFC-9381](https://datatracker.ietf.org/doc/rfc9381).
//!
//! The implementation extends RFC9381 to allow to sign additional user data together
//! with the VRF input. Refer to https://github.com/davxy/bandersnatch-vrfs-spec for
//! specification extension details.

use super::*;

pub trait IetfSuite: Suite {}

impl<T> IetfSuite for T where T: Suite {}

/// VRF proof generic over the cipher suite.
///
/// An output point which can be used to derive the actual output together
/// with the actual proof of the input point and the associated data.
#[derive(Debug, Clone)]
pub struct Proof<S: IetfSuite> {
    pub c: ScalarField<S>,
    pub s: ScalarField<S>,
}

impl<S: IetfSuite> CanonicalSerialize for Proof<S> {
    fn serialize_with_mode<W: ark_serialize::Write>(
        &self,
        mut writer: W,
        _compress_always: ark_serialize::Compress,
    ) -> Result<(), ark_serialize::SerializationError> {
        let buf = utils::encode_scalar::<S>(&self.c);
        if buf.len() < S::CHALLENGE_LEN {
            // Encoded scalar length must be at least S::CHALLENGE_LEN
            return Err(ark_serialize::SerializationError::NotEnoughSpace);
        }
        writer.write_all(&buf[..S::CHALLENGE_LEN])?;
        self.s.serialize_compressed(&mut writer)?;
        Ok(())
    }

    fn serialized_size(&self, _compress_always: ark_serialize::Compress) -> usize {
        S::CHALLENGE_LEN + self.s.compressed_size()
    }
}

impl<S: IetfSuite> CanonicalDeserialize for Proof<S> {
    fn deserialize_with_mode<R: ark_serialize::Read>(
        mut reader: R,
        _compress_always: ark_serialize::Compress,
        validate: ark_serialize::Validate,
    ) -> Result<Self, ark_serialize::SerializationError> {
        let c = <ScalarField<S> as CanonicalDeserialize>::deserialize_with_mode(
            &mut reader,
            ark_serialize::Compress::No,
            validate,
        )?;
        let s = <ScalarField<S> as CanonicalDeserialize>::deserialize_with_mode(
            &mut reader,
            ark_serialize::Compress::No,
            validate,
        )?;
        Ok(Proof { c, s })
    }
}

impl<S: IetfSuite> ark_serialize::Valid for Proof<S> {
    fn check(&self) -> Result<(), ark_serialize::SerializationError> {
        self.c.check()?;
        self.s.check()?;
        Ok(())
    }
}

pub trait Prover<S: IetfSuite> {
    /// Generate a proof for the given input/output and user additional data.
    fn prove(&self, input: Input<S>, output: Output<S>, ad: impl AsRef<[u8]>) -> Proof<S>;
}

pub trait Verifier<S: IetfSuite> {
    /// Verify a proof for the given input/output and user additional data.
    fn verify(
        &self,
        input: Input<S>,
        output: Output<S>,
        ad: impl AsRef<[u8]>,
        sig: &Proof<S>,
    ) -> Result<(), Error>;
}

impl<S: IetfSuite> Prover<S> for Secret<S> {
    fn prove(&self, input: Input<S>, output: Output<S>, ad: impl AsRef<[u8]>) -> Proof<S> {
        let k = S::nonce(&self.scalar, input);
        let k_b = (S::Affine::generator() * k).into_affine();

        let k_h = (input.0 * k).into_affine();

        let c = S::challenge(
            &[&self.public.0, &input.0, &output.0, &k_b, &k_h],
            ad.as_ref(),
        );
        let s = k + c * self.scalar;
        Proof { c, s }
    }
}

impl<S: IetfSuite> Verifier<S> for Public<S> {
    fn verify(
        &self,
        input: Input<S>,
        output: Output<S>,
        ad: impl AsRef<[u8]>,
        proof: &Proof<S>,
    ) -> Result<(), Error> {
        let Proof { c, s } = proof;

        let s_b = S::Affine::generator() * s;
        let c_y = self.0 * c;
        let u = (s_b - c_y).into_affine();

        let s_h = input.0 * s;
        let c_o = output.0 * c;
        let v = (s_h - c_o).into_affine();

        let c_exp = S::challenge(&[&self.0, &input.0, &output.0, &u, &v], ad.as_ref());
        (&c_exp == c)
            .then_some(())
            .ok_or(Error::VerificationFailure)
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use crate::testing as common;

    pub struct TestVector<S: IetfSuite> {
        pub base: common::TestVector<S>,
        pub c: ScalarField<S>,
        pub s: ScalarField<S>,
    }

    impl<S: IetfSuite> core::fmt::Debug for TestVector<S> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let c = hex::encode(utils::encode_scalar::<S>(&self.c));
            let s = hex::encode(utils::encode_scalar::<S>(&self.s));
            f.debug_struct("TestVector")
                .field("base", &self.base)
                .field("proof_c", &c)
                .field("proof_s", &s)
                .finish()
        }
    }

    impl<S: IetfSuite + std::fmt::Debug> common::TestVectorTrait for TestVector<S> {
        fn new(
            comment: &str,
            seed: &[u8],
            alpha: &[u8],
            salt: Option<&[u8]>,
            ad: &[u8],
            flags: u8,
        ) -> Self {
            use super::Prover;
            let base = common::TestVector::new(comment, seed, alpha, salt, ad, flags);
            // TODO: store constructed types in the vectors
            let input = Input::from(base.h);
            let output = Output::from(base.gamma);
            let sk = Secret::from_scalar(base.sk);
            let proof: Proof<S> = sk.prove(input, output, ad);
            Self {
                base,
                c: proof.c,
                s: proof.s,
            }
        }

        fn from_map(map: &common::TestVectorMap) -> Self {
            let base = common::TestVector::from_map(map);
            let c = utils::decode_scalar::<S>(&map.item_bytes("proof_c"));
            let s = utils::decode_scalar::<S>(&map.item_bytes("proof_s"));
            Self { base, c, s }
        }

        fn to_map(&self) -> common::TestVectorMap {
            let items = [
                ("proof_c", hex::encode(utils::encode_scalar::<S>(&self.c))),
                ("proof_s", hex::encode(utils::encode_scalar::<S>(&self.s))),
            ];
            let mut map = self.base.to_map();
            items.into_iter().for_each(|(name, value)| {
                map.0.insert(name.to_string(), value);
            });
            map
        }

        fn run(&self) {
            self.base.run();
            if self.base.flags & common::TEST_FLAG_SKIP_PROOF_CHECK != 0 {
                return;
            }
            let input = Input::<S>::from(self.base.h);
            let output = Output::from(self.base.gamma);
            let sk = Secret::from_scalar(self.base.sk);
            let proof = sk.prove(input, output, &self.base.ad);
            assert_eq!(self.c, proof.c, "VRF proof challenge ('c') mismatch");
            assert_eq!(self.s, proof.s, "VRF proof response ('s') mismatch");

            let pk = Public(self.base.pk);
            assert!(pk.verify(input, output, &self.base.ad, &proof).is_ok());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        random_val,
        suite::{AffinePoint, Input, ScalarField, Secret, TestSuite},
        TEST_SEED,
    };

    #[test]
    fn prove_verify_works() {
        let secret = Secret::from_seed(TEST_SEED);
        let public = secret.public();
        let input = Input::from(random_val::<AffinePoint>(None));
        let output = secret.output(input);

        let proof = secret.prove(input, output, b"foo");

        let result = public.verify(input, output, b"foo", &proof);
        assert!(result.is_ok());
    }

    #[test]
    fn proof_encode_decode() {
        let c = hex::decode("d091c00b0f5c3619d10ecea44363b5a5").unwrap();
        let c = ScalarField::from_be_bytes_mod_order(&c[..]);
        let s = hex::decode("99cadc5b2957e223fec62e81f7b4825fc799a771a3d7334b9186bdbee87316b1")
            .unwrap();
        let s = ScalarField::from_be_bytes_mod_order(&s[..]);

        let proof = Proof::<TestSuite> { c, s };

        let mut buf = Vec::new();
        proof.serialize_compressed(&mut buf).unwrap();
        assert_eq!(buf.len(), TestSuite::CHALLENGE_LEN + 32);
    }
}
