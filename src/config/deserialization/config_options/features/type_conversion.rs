use super::{FeaturesCompleteState, FeaturesWithCommas};
use os_checker_types::config as out;

impl From<FeaturesCompleteState> for out::FeaturesCompleteState {
    fn from(
        FeaturesCompleteState {
            f,
            no_default_features,
            all_features,
            targets,
        }: FeaturesCompleteState,
    ) -> Self {
        Self {
            f: f.into(),
            no_default_features,
            all_features,
            targets,
        }
    }
}

impl From<FeaturesWithCommas> for out::FeaturesWithCommas {
    fn from(value: FeaturesWithCommas) -> Self {
        Self {
            features: value.features,
        }
    }
}

impl From<out::FeaturesCompleteState> for FeaturesCompleteState {
    fn from(
        out::FeaturesCompleteState {
            f,
            no_default_features,
            all_features,
            targets,
        }: out::FeaturesCompleteState,
    ) -> Self {
        Self {
            f: f.into(),
            no_default_features,
            all_features,
            targets,
        }
    }
}

impl From<out::FeaturesWithCommas> for FeaturesWithCommas {
    fn from(value: out::FeaturesWithCommas) -> Self {
        Self {
            features: value.features,
        }
    }
}
