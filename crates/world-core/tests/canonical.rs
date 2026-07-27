use world_core::{
    CANONICAL_PROTOCOL_IDENTIFIER, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    DigestAlgorithm, SELECTED_DIGEST_ALGORITHM,
};

const TEST_DOMAIN: CanonicalDomain = match CanonicalDomain::new("world-test-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("test domain must be valid"),
};
const FIRST_DOMAIN: CanonicalDomain = match CanonicalDomain::new("world-first-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("test domain must be valid"),
};
const SECOND_DOMAIN: CanonicalDomain = match CanonicalDomain::new("world-second-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("test domain must be valid"),
};
const TEXT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("world-text-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("test domain must be valid"),
};

#[test]
fn canonical_domains_use_a_bounded_stable_alphabet() {
    assert_eq!(CanonicalDomain::new(""), Err(CanonicalError::EmptyDomain));
    assert!(matches!(
        CanonicalDomain::new("World-record"),
        Err(CanonicalError::InvalidDomainByte {
            index: 0,
            byte: b'W'
        })
    ));
    assert!(matches!(
        CanonicalDomain::new("1world-record"),
        Err(CanonicalError::InvalidDomainByte {
            index: 0,
            byte: b'1'
        })
    ));
    assert!(matches!(
        CanonicalDomain::new("world_record"),
        Err(CanonicalError::InvalidDomainByte {
            index: 5,
            byte: b'_'
        })
    ));
    assert!(matches!(
        CanonicalDomain::new("world-é-v1"),
        Err(CanonicalError::InvalidDomainByte {
            index: 6,
            byte: 0xc3
        })
    ));
    assert!(
        CanonicalDomain::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .is_ok()
    );
    assert!(matches!(
        CanonicalDomain::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        Err(CanonicalError::DomainTooLong {
            length: 65,
            maximum: 64
        })
    ));
}

#[test]
fn canonical_writer_matches_the_frozen_big_endian_vector() {
    let mut writer = CanonicalWriter::new(TEST_DOMAIN);
    writer.write_u8(0x7f);
    writer.write_u16(0x1234);
    writer.write_u32(0x5678_9abc);
    writer.write_u64(0xdef0_1234_5678_9abc);
    writer.write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
    writer.write_bool(false);
    writer.write_bool(true);
    writer.write_discriminant(3);
    assert_eq!(writer.write_bytes(&[0, 0xff]), Ok(()));
    assert_eq!(writer.write_str("é"), Ok(()));
    assert_eq!(
        writer.write_option(Some(&9_u16), |writer, value| {
            writer.write_u16(*value);
            Ok(())
        }),
        Ok(())
    );
    assert_eq!(
        writer.write_sequence(&[5_u8, 6], |writer, value| {
            writer.write_u8(*value);
            Ok(())
        }),
        Ok(())
    );

    let bytes = writer.finish();
    const EXPECTED: &[u8] = &[
        119, 111, 114, 108, 100, 45, 99, 97, 110, 111, 110, 105, 99, 97, 108, 45, 118, 49, 0, 0, 0,
        0, 0, 0, 0, 13, 119, 111, 114, 108, 100, 45, 116, 101, 115, 116, 45, 118, 49, 127, 18, 52,
        86, 120, 154, 188, 222, 240, 18, 52, 86, 120, 154, 188, 1, 35, 69, 103, 137, 171, 205, 239,
        254, 220, 186, 152, 118, 84, 50, 16, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 255, 0,
        0, 0, 0, 0, 0, 0, 2, 195, 169, 1, 0, 9, 0, 0, 0, 0, 0, 0, 0, 2, 5, 6,
    ];

    assert_eq!(
        CANONICAL_PROTOCOL_IDENTIFIER.as_bytes(),
        &EXPECTED[..CANONICAL_PROTOCOL_IDENTIFIER.len()]
    );
    assert_eq!(bytes.as_bytes(), EXPECTED);
    assert_eq!(
        ContentDigest::of_canonical(&bytes).to_string(),
        "ce51ee3c303921a7f271a55a0351903333277412aaed8d142ef37e876c44d878"
    );
    assert_eq!(bytes.into_bytes(), EXPECTED);
}

#[test]
fn equal_fields_in_distinct_domains_have_distinct_identity() {
    let mut first = CanonicalWriter::new(FIRST_DOMAIN);
    let mut second = CanonicalWriter::new(SECOND_DOMAIN);
    first.write_u64(7);
    second.write_u64(7);

    let first = first.finish();
    let second = second.finish();
    assert_ne!(first, second);
    assert_ne!(
        ContentDigest::of_canonical(&first),
        ContentDigest::of_canonical(&second)
    );
}

#[test]
fn canonical_strings_preserve_exact_utf8_without_normalization() {
    let mut composed = CanonicalWriter::new(TEXT_DOMAIN);
    let mut decomposed = CanonicalWriter::new(TEXT_DOMAIN);
    assert_eq!(composed.write_str("é"), Ok(()));
    assert_eq!(decomposed.write_str("e\u{301}"), Ok(()));

    let composed = composed.finish();
    let decomposed = decomposed.finish();
    assert_ne!(composed, decomposed);
    assert_ne!(
        ContentDigest::of_canonical(&composed),
        ContentDigest::of_canonical(&decomposed)
    );
}

#[test]
fn option_and_sequence_framing_prevents_boundary_ambiguity() {
    let mut absent = CanonicalWriter::new(TEST_DOMAIN);
    let mut present_zero = CanonicalWriter::new(TEST_DOMAIN);
    assert_eq!(
        absent.write_option::<u8>(None, |writer, value| {
            writer.write_u8(*value);
            Ok(())
        }),
        Ok(())
    );
    assert_eq!(
        present_zero.write_option(Some(&0), |writer, value| {
            writer.write_u8(*value);
            Ok(())
        }),
        Ok(())
    );
    let absent = absent.finish();
    let present_zero = present_zero.finish();
    assert_eq!(absent.as_bytes().last(), Some(&0));
    assert_eq!(
        &present_zero.as_bytes()[present_zero.as_bytes().len() - 2..],
        &[1, 0]
    );
    assert_ne!(absent, present_zero);

    let mut joined_left = CanonicalWriter::new(TEST_DOMAIN);
    let mut joined_right = CanonicalWriter::new(TEST_DOMAIN);
    assert_eq!(
        joined_left.write_sequence(&["ab", "c"], |writer, value| writer.write_str(value)),
        Ok(())
    );
    assert_eq!(
        joined_right.write_sequence(&["a", "bc"], |writer, value| writer.write_str(value)),
        Ok(())
    );
    assert_ne!(joined_left.finish(), joined_right.finish());

    let mut empty = CanonicalWriter::new(TEST_DOMAIN);
    let mut one_empty = CanonicalWriter::new(TEST_DOMAIN);
    assert_eq!(
        empty.write_sequence::<&str>(&[], |writer, value| writer.write_str(value)),
        Ok(())
    );
    assert_eq!(
        one_empty.write_sequence(&[""], |writer, value| writer.write_str(value)),
        Ok(())
    );
    assert_ne!(empty.finish(), one_empty.finish());
}

#[test]
fn failed_container_writes_restore_the_previous_preimage() {
    let injected = CanonicalError::LengthOverflow { length: usize::MAX };

    let mut option = CanonicalWriter::new(TEST_DOMAIN);
    option.write_u8(7);
    assert_eq!(
        option.write_option(Some(&9_u8), |writer, value| {
            writer.write_u8(*value);
            Err(injected)
        }),
        Err(injected)
    );

    let mut sequence = CanonicalWriter::new(TEST_DOMAIN);
    sequence.write_u8(7);
    assert_eq!(
        sequence.write_sequence(&[9_u8], |writer, value| {
            writer.write_u8(*value);
            Err(injected)
        }),
        Err(injected)
    );

    let mut expected = CanonicalWriter::new(TEST_DOMAIN);
    expected.write_u8(7);
    let expected = expected.finish();
    assert_eq!(option.finish(), expected);
    assert_eq!(sequence.finish(), expected);
}

#[test]
fn selected_hash_matches_the_official_blake3_empty_input_vector() {
    assert_eq!(SELECTED_DIGEST_ALGORITHM, DigestAlgorithm::Blake3_256);
    assert_eq!(SELECTED_DIGEST_ALGORITHM.identifier(), "blake3-256");
    assert_eq!(
        ContentDigest::of_blob_bytes(b"").to_string(),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
}

#[test]
fn canonical_digest_has_stable_display_and_raw_ordering() {
    let digest = ContentDigest::of_blob_bytes(b"world");
    let encoded = digest.to_string();

    assert_eq!(encoded.len(), 64);
    assert!(encoded.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(encoded, encoded.to_ascii_lowercase());
    assert!(
        ContentDigest::from_bytes([0; 32]) < ContentDigest::from_bytes([0xff; 32]),
        "digest ordering follows raw bytes"
    );
}
