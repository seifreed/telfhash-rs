use crate::domain::error::TelfhashError;
use crate::domain::model::{HashValue, NullDigestReason};
use crate::domain::ports::SimilarityHasher;
use crate::infrastructure::telemetry::debug;
use tlsh_rs::{TlshDigest, TlshError, hash_bytes};

pub struct TlshRsHasher;

impl SimilarityHasher for TlshRsHasher {
    fn hash_symbols(&self, symbols: &[String]) -> Result<HashValue, TelfhashError> {
        let payload = symbols.join(",");
        debug!(
            symbol_count = symbols.len(),
            payload_len = payload.len(),
            "building TLSH payload"
        );
        match hash_bytes(payload.as_bytes()) {
            Ok(digest) => {
                debug!("TLSH digest generated");
                Ok(HashValue::Digest(digest.encoded()))
            }
            Err(TlshError::TooShort { .. } | TlshError::InsufficientVariance) => {
                debug!("TLSH returned TNULL");
                Ok(HashValue::NullDigest(
                    NullDigestReason::InsufficientInformation,
                ))
            }
            Err(error) => Err(TelfhashError::TlshGeneration(error.to_string())),
        }
    }

    fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError> {
        debug!("computing TLSH distance");
        let left = TlshDigest::from_encoded(&left.to_ascii_uppercase())
            .map_err(|error| TelfhashError::TlshComparison(error.to_string()))?;
        let right = TlshDigest::from_encoded(&right.to_ascii_uppercase())
            .map_err(|error| TelfhashError::TlshComparison(error.to_string()))?;

        left.try_diff(&right)
            .map(|value| value as u32)
            .map_err(|error| TelfhashError::TlshComparison(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::model::{HashValue, NullDigestReason};
    use crate::domain::ports::SimilarityHasher;

    use super::TlshRsHasher;

    #[test]
    fn generates_and_compares_hashes() {
        let hasher = TlshRsHasher;
        let symbols = (0..80)
            .map(|index| format!("symbol_{index}"))
            .collect::<Vec<_>>();
        let hash = hasher.hash_symbols(&symbols).unwrap();

        let HashValue::Digest(hash) = hash else {
            panic!("expected digest")
        };
        assert_eq!(hash.len(), 72);
        assert!(hash.starts_with("T1"));
        assert_eq!(hasher.distance(&hash, &hash).unwrap(), 0);
    }

    #[test]
    fn returns_tnull_for_low_information_inputs() {
        let hasher = TlshRsHasher;
        let symbols = vec!["ceilf".to_string(), "nextafter".to_string()];

        assert_eq!(
            hasher.hash_symbols(&symbols).unwrap(),
            HashValue::NullDigest(NullDigestReason::InsufficientInformation)
        );
    }
}
