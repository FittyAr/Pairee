//! Known-answer tests for every transfer hash algorithm.

use super::create_hasher;
use crate::fs::transfer::options::HashAlgorithm;

/// `(algorithm, digest of "", digest of "abc")`, upper-case hex as produced
/// by the hashers. Values come from the algorithms' reference test vectors.
const VECTORS: &[(HashAlgorithm, &str, &str)] = &[
    (HashAlgorithm::Crc32, "00000000", "352441C2"),
    (
        HashAlgorithm::Md5,
        "D41D8CD98F00B204E9800998ECF8427E",
        "900150983CD24FB0D6963F7D28E17F72",
    ),
    (
        HashAlgorithm::Sha1,
        "DA39A3EE5E6B4B0D3255BFEF95601890AFD80709",
        "A9993E364706816ABA3E25717850C26C9CD0D89D",
    ),
    (
        HashAlgorithm::Sha256,
        "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
    ),
    (
        HashAlgorithm::Blake3,
        "AF1349B9F5F9A1A6A0404DEA36DCC9499BCB25C9ADC112B7CC9A93CAE41F3262",
        "6437B3AC38465133FFB63B75273A8DB548C558465D79DB03FD359C6CD5BD9D85",
    ),
];

fn digest(algorithm: HashAlgorithm, chunks: &[&[u8]]) -> String {
    let mut hasher = create_hasher(algorithm);
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize()
}

#[test]
fn empty_input_matches_reference_digest() {
    for (algorithm, empty, _) in VECTORS {
        assert_eq!(digest(*algorithm, &[]), *empty, "{algorithm:?}");
    }
}

#[test]
fn abc_matches_reference_digest() {
    for (algorithm, _, abc) in VECTORS {
        assert_eq!(digest(*algorithm, &[b"abc"]), *abc, "{algorithm:?}");
    }
}

#[test]
fn chunked_updates_equal_single_update() {
    for (algorithm, _, abc) in VECTORS {
        assert_eq!(
            digest(*algorithm, &[b"a", b"", b"bc"]),
            *abc,
            "{algorithm:?}"
        );
    }
}

#[test]
fn crc32_check_value() {
    // Standard CRC-32/ISO-HDLC check value for "123456789".
    assert_eq!(digest(HashAlgorithm::Crc32, &[b"123456789"]), "CBF43926");
}
