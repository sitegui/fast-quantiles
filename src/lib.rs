#[cfg(test)]
mod tests;

pub mod mean;

pub mod quantile_generator;

pub mod gk;

pub trait Operation {
    type Item;
    type Output;

    fn create() -> Self;

    fn update(&mut self, value: Self::Item);

    fn merge_with(&mut self, other: Self);

    fn finish(self) -> Self::Output;
}
