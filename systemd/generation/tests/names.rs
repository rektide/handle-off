use handle_off_systemd_generation::*;

#[test]
fn typed_tokens_and_all_address_roles_round_trip_with_exact_manager_bounds() {
    for value in [0, 1, u128::MAX, 0x0123456789abcdef] {
        let namespace = NamespaceToken::from_u128(value);
        let generation = GenerationToken::from_bytes([0xab; 16]);
        assert_eq!(
            NamespaceToken::parse(&namespace.to_string()).unwrap(),
            namespace
        );
        assert_eq!(
            GenerationToken::parse(&generation.to_string()).unwrap(),
            generation
        );
        for member in [
            Member::Root,
            Member::CommitMarker,
            Member::Resource(ResourceSlot::new(0)),
            Member::Resource(ResourceSlot::new(u32::MAX)),
        ] {
            let address = Address {
                key: GenerationKey {
                    namespace,
                    generation,
                },
                member,
            };
            let name = address.to_string();
            assert_eq!(Address::parse(&name).unwrap(), Some(address));
            assert!(matches!(name.len(), 71 | 79));
            assert!(name.len() <= 255);
            assert!(
                name.bytes()
                    .all(|byte| (32..127).contains(&byte) && byte != b':')
            );
        }
    }
}
#[test]
fn wrong_width_uppercase_bad_delimiters_and_slot_overflow_fail_without_normalization() {
    for token in [
        "",
        "0",
        "0123456789abcdef0123456789abcdef0",
        "ABCDEF0123456789ABCDEF0123456789AB",
        "gggggggggggggggggggggggggggggggg",
        "🐟🐟🐟🐟🐟🐟🐟🐟",
    ] {
        assert!(NamespaceToken::parse(token).is_err());
    }
    let prefix = "ho1.00000000000000000000000000000001.00000000000000000000000000000002.";
    for suffix in [
        "",
        "M",
        "root",
        "r0",
        "r100000000",
        "rfffffffff",
        "r0000000A",
        "r0000000g",
        "m.extra",
    ] {
        assert!(
            Address::parse(&format!("{prefix}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    let mut bad = format!("{prefix}m").into_bytes();
    bad[36] = b'/';
    assert_eq!(
        Address::parse(std::str::from_utf8(&bad).unwrap()),
        Err(NameError::Delimiter)
    );
    assert!(Address::parse("ho1").is_err());
    assert!(Address::parse(&format!("ho1.{}", "x".repeat(1_000_000))).is_err());
    for name in ["listener", "", "ho2.future", "HO1.unrelated", "ho1x"] {
        assert_eq!(Address::parse(name).unwrap(), None);
    }
}
