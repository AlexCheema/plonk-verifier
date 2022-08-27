use core::num;
use std::vec;

use group::Curve;

use crate::{
    loader::{LoadedScalar, Loader},
    scheme::kzg::accumulation::{AccumulationScheme, AccumulationStrategy, Accumulator},
    util::TranscriptRead,
};

struct VerificationKey<C: Curve, L: Loader<C>> {
    // All Loaded values for the verification key.
    // I think you can shit Domain here as well, but don't worry about it right now
    domain: Domain<C>,
    public_inputs_count: usize,
    k1: L::LoadedScalar,
    k2: L::LoadedScalar,
    Qm: L::LoadedEcPoint,
    Ql: L::LoadedEcPoint,
    Qr: L::LoadedEcPoint,
    Qo: L::LoadedEcPoint,
    Qc: L::LoadedEcPoint,
    S1: L::LoadedEcPoint,
    S2: L::LoadedEcPoint,
    S3: L::LoadedEcPoint,
    X_2: L::LoadedEcPoint,
    // Contains omega^j `for j in range(0, public_inputs.length)`.
    // This is to avoid more constraints.
    omegas: Vec<L::LoadedScalar>,
    // all `omegas` inversed
    omegas_inv: Vec<LoadedScalar>,
}

impl<C: Curve, L: Loader<C>> VerificationKey<C, L> {
    pub fn read() -> Self {
        todo!()
    }
}

pub struct Challenges<C: Curve, L: Loader<C>> {
    beta: L::LoadedScalar,
    alpha: L::LoadedScalar,
    gamma: L::LoadedScalar,
    xi: L::LoadedScalar,
    v: L::LoadedScalar,
    u: L::LoadedScalar,
}

pub struct CircomPlonkProof<C: Curve, L: Loader<C>> {
    public_signals: Vec<L::LoadedScalar>,
    A: L::LoadedEcPoint,
    B: L::LoadedEcPoint,
    C: L::LoadedEcPoint,
    Z: L::LoadedEcPoint,
    T1: L::LoadedEcPoint,
    T2: L::LoadedEcPoint,
    T3: L::LoadedEcPoint,
    Wxi: L::LoadedEcPoint,
    Wxiw: L::LoadedEcPoint,
    eval_a: L::LoadedScalar,
    eval_b: L::LoadedScalar,
    eval_c: L::LoadedScalar,
    eval_s1: L::LoadedScalar,
    eval_s2: L::LoadedScalar,
    eval_zw: L::LoadedScalar,
    eval_r: L::LoadedScalar,
    challenges: Challenges<C, L>,
}

pub struct Domain<C: Curve> {
    k: usize,
    n: usize,
    omega: C::Scalar,
    omega_inv: C::Scalar,
}

impl<C: Curve> Domain<C> {
    pub fn new(k: u32) -> Self {
        assert!(k < C::Scalar::S as usize);

        let n = 1 << k;
        let omega = C::Scalar::root_of_unity();

        Self {
            k,
            n,
            omega,
            omega_inv: omega.invert(),
        }
    }
}

// 1. Aggregation challenge can be obtained from encoding proofs
// 2. Simply use different powers of aggregation challenge `a`
// 3. Note that in step 9 onwards, just accumulate the scalars but dont perform `Scalar multiplication`. Delay MSM till the end.

impl<C: Curve, L: Loader<C>> CircomPlonkProof<C, L> {
    fn read<T: TranscriptRead<C, L>>(
        public_signals: &Vec<L::LoadedScalar>,
        transcript: &mut T,
    ) -> Result<Self, Error> {
        public_signals
            .iter()
            .for_each(|signal| transcript.common_scalar(signal));

        let A = transcript.read_ec_point()?;
        let B = transcript.read_ec_point()?;
        let C = transcript.read_ec_point()?;

        let beta = transcript.squeeze_challenge();

        transcript.common_scalar(beta);
        let gamma = transcript.squeeze_challenge();

        let Z = transcript.read_ec_point()?;
        let alpha = transcript.squeeze_challenge();

        let T1 = transcript.read_ec_point()?;
        let T2 = transcript.read_ec_point()?;
        let T3 = transcript.read_ec_point()?;
        let xi = transcript.squeeze_challenge();

        let eval_points: [L::LoadedScalar; 7] = transcript.read_n_scalars(7).into()?;
        let v = transcript.squeeze_challenge();

        let Wxi = transcript.read_ec_point()?;
        let Wxiw = transcript.read_ec_point()?;
        let u = transcript.squeeze_challenge();

        Ok(Self {
            public_signals,
            A,
            B,
            C,
            Z,
            T1,
            T2,
            T3,
            Wxi,
            Wxiw,
            eval_a: eval_points[0],
            eval_b: eval_points[1],
            eval_c: eval_points[2],
            eval_s1: eval_points[3],
            eval_s2: eval_points[4],
            eval_zw: eval_points[5],
            eval_r: eval_points[6],
            challenges: Challenges {
                beta,
                alpha,
                gamma,
                xi,
                v,
                u,
            },
        })
    }
}

#[derive(Default)]
pub struct CircomPlonkAccumulationScheme;

impl<C, L, T, S> AccumulationScheme for CircomPlonkAccumulationScheme
where
    C: Curve,
    L: Loader,
    T: TranscriptRead<C, L>,
    S: AccumulationStrategy<C, L, CircomPlonkProof<C, L>>,
{
    type Proof = CircomPlonkProof<C, L>;

    fn accumulate(
        protocol: &crate::protocol::Protocol<C>,
        vk_key: &VerificationKey<C, L>,
        loader: &L,
        public_signals: &Vec<L::LoadedScalar>,
        transcript: &mut T,
        strategy: &mut S,
    ) -> Result<S::Output, crate::Error> {
        // perform necessary checks
        // 1. check that public signals are of correct length
        // 2  check that omegas length in `vk` match public inputs length

        let proof = CircomPlonkProof::read(public_signals, transcript)?;

        // xi^n
        let xi = proof.challenges.xi;
        let xi_power_n = xi.pow_constant(vk_key.domain.n);

        // z_h(xi) = xi^n - 1;
        let one = loader.load_const(C::Scalar::one());
        let z_h_eval_xi = xi_power_n - one;

        // Compute first lagrange evaluation.
        // Snarkjs's plonk prover starts with `omega^0`
        // in permutation polynomial. Thus we compute
        // `L0(xi)` here.
        //
        // `L0(xi) = (xi^n) - 1 / (n * (xi - 1))`
        //
        // More info on this - https://github.com/ZK-Garage/plonk/blob/79dffa1bacbe73ab42e2d7e48194efe5c0070bd6/plonk-core/src/proof_system/proof.rs#L622
        let l1_eval_xi = {
            let denom = xi - one;
            z_h_eval_xi * denom.invert()
        };

        // Compute public input poly evaluation at `xi`.
        // We do this using `barycentric evaluation` approach.
        // For more details on this approach check following:
        //  (1) https://hackmd.io/@vbuterin/barycentric_evaluation
        //  (2) https://github.com/ZK-Garage/plonk/blob/79dffa1bacbe73ab42e2d7e48194efe5c0070bd6/plonk-core/src/proof_system/proof.rs#L635
        //
        // TODO: We store `omegas` in `vk`. We only need them at this
        // step of verification. This means we shall only load omages
        // omegas_inv for range (0..public_inputs.length). Implement this
        // optimization.
        let pi_poly_xi = {
            // (xi^n - 1) / n
            //
            // TODO: store `n.invert()` in `vk` to avoid
            // havint to constrain it in every accumulation step.
            let numerator = z_h_eval_xi * n.invert();

            if public_signals.len() == 0 {
                numerator
            } else {
                let denominator = {
                    let denoms = (0..public_signals.len())
                        .map(|index| {
                            // (xi - omega^i) * omega^-1 => (omega^-1 * xi - 1)
                            let d = xi * vk_key.omegas_inv[index].unwrap();
                            let d = d - one;
                            d
                        })
                        .collect();
                    let denoms = loader.batch_invert(denoms);

                    let mut sum = denoms[0] * public_signals[0];
                    denoms
                        .iter()
                        .skip(1)
                        .chain(public_signals.iter().skip(1))
                        .for_each(|d, pi| {
                            let ith_val = d * pi;
                            sum += ith_val;
                        });
                    sum
                };
                numerator * denominator
            }
        };

        todo!()
    }
}
