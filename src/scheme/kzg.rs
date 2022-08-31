use crate::{
    protocol::Protocol,
    util::{Curve, Expression},
};

mod accumulation;
mod cost;
mod msm;

pub use accumulation::{
    circom_plonk::{CircomPlonkAccumulationScheme, CircomPlonkProof, Protocol as CircomProtocol},
    plonk::PlonkAccumulationScheme,
    shplonk::ShplonkAccumulationScheme,
    AccumulationScheme, AccumulationStrategy, Accumulator, SameCurveAccumulation,
};
pub use cost::{Cost, CostEstimation};
pub use msm::MSM;

pub fn langranges<C: Curve, T>(
    protocol: &Protocol<C>,
    statements: &[Vec<T>],
) -> impl IntoIterator<Item = i32> {
    protocol
        .relations
        .iter()
        .cloned()
        .sum::<Expression<_>>()
        .used_langrange()
        .into_iter()
        .chain(
            0..statements
                .iter()
                .map(|statement| statement.len())
                .max()
                .unwrap_or_default() as i32,
        )
}
