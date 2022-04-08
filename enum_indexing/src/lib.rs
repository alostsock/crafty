trait EnumIndexing {
    fn index(&self) -> usize;
    fn from_index(index: usize) -> Option<Self>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_indexing_derive::EnumIndexing;

    #[derive(EnumIndexing, Debug, PartialEq, Eq, Clone)]
    enum TestEnum {
        A,
        B,
        C,
    }

    const VARIANTS: &[TestEnum] = &[TestEnum::A, TestEnum::B, TestEnum::C];
    const INDICES: &[usize] = &[0, 1, 2];

    #[test]
    fn index_works() {
        let indices: Vec<usize> = VARIANTS.iter().map(|e| e.index()).collect();
        assert_eq!(indices, INDICES.to_vec());
    }

    #[test]
    fn from_index_works() {
        let variants: Vec<TestEnum> = INDICES
            .iter()
            .map(|i| TestEnum::from_index(*i).unwrap())
            .collect();
        assert_eq!(variants, VARIANTS.to_vec());
    }
}
