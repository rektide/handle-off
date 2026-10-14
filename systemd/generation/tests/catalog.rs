use handle_off_bundle::{
    Bundle, BundleId, GenerationId, MANIFEST_VERSION, Manifest, Member as LogicalMember, MemberId,
};
use handle_off_handles::ResourceId;
use handle_off_state::{Extension, Payload, StateFormat};
use handle_off_systemd_generation::*;
use std::{cell::Cell, rc::Rc};

fn key(namespace: u128, generation: u128) -> GenerationKey {
    GenerationKey {
        namespace: NamespaceToken::from_u128(namespace),
        generation: GenerationToken::from_u128(generation),
    }
}
fn name(key: GenerationKey, member: Member) -> String {
    Address { key, member }.to_string()
}
fn discover<'a, H>(entries: &'a [(Option<String>, H)]) -> Result<Catalog<'a, H>, CatalogError> {
    Catalog::discover(
        entries.iter().map(|(name, handle)| NamedHandle {
            name: name.as_deref(),
            handle,
        }),
        Limits::default(),
    )
}
fn manifest(ids: &[&str]) -> Manifest {
    Manifest {
        version: MANIFEST_VERSION,
        generation: GenerationId::new("arbitrary logical generation 🐟").unwrap(),
        resources: ids.iter().map(|id| ResourceId::new(*id).unwrap()).collect(),
        bundles: ids
            .iter()
            .enumerate()
            .map(|(index, id)| Bundle {
                id: BundleId::new(format!("application-bundle-{index}")).unwrap(),
                members: vec![LogicalMember {
                    id: MemberId::new("state").unwrap(),
                    resource: ResourceId::new(*id).unwrap(),
                    extensions: vec![Extension {
                        key: "checkpoint".into(),
                        format: StateFormat {
                            name: "app.state".into(),
                            version: 1,
                        },
                        required: true,
                        payload: Payload::ResourceRange {
                            resource: ResourceId::new(*id).unwrap(),
                            offset: 0,
                            length: 4,
                        },
                    }],
                }],
                extensions: vec![],
            })
            .collect(),
        extensions: vec![],
    }
}
#[test]
fn shuffled_multiple_generations_and_namespaces_are_never_chosen_or_merged() {
    let first = key(1, 20);
    let second = key(1, 10);
    let foreign = key(2, 20);
    let entries = vec![
        (
            Some(name(first, Member::Resource(ResourceSlot::new(1)))),
            101,
        ),
        (Some("listener".into()), 900),
        (Some(name(second, Member::Root)), 200),
        (Some(name(first, Member::CommitMarker)), 103),
        (Some(name(foreign, Member::Root)), 300),
        (Some(name(first, Member::Root)), 100),
        (None, 901),
        (
            Some(name(first, Member::Resource(ResourceSlot::new(0)))),
            102,
        ),
        (Some("ho2.future-format".into()), 902),
    ];
    let catalog = discover(&entries).unwrap();
    assert_eq!(catalog.candidates().len(), 3);
    assert_eq!(catalog.namespace(first.namespace).count(), 2);
    assert_eq!(
        *catalog.candidate(second).unwrap().root().unwrap().handle,
        200
    );
    assert_eq!(
        *catalog.candidate(foreign).unwrap().root().unwrap().handle,
        300
    );
    let candidate = catalog.candidate(first).unwrap();
    let logical = manifest(&["image-b", "image-a"]);
    let mapping = candidate.map_manifest(&logical).unwrap();
    assert_eq!(mapping.key, first);
    assert_eq!(mapping.logical_generation, &logical.generation);
    assert_eq!(mapping.resources[0].resource_id.as_str(), "image-b");
    assert_eq!(*mapping.resources[0].handle, 102);
    assert_eq!(mapping.resources[1].resource_id.as_str(), "image-a");
    assert_eq!(*mapping.resources[1].handle, 101);
    assert_eq!(mapping.commit_marker, Some(&103));
    assert_eq!(catalog.unrelated().len(), 3);
    assert_eq!(catalog.unrelated()[0].name, Some("listener"));
    assert_eq!(*catalog.unrelated()[0].handle, 900);
}
#[test]
fn duplicate_recognized_names_fail_even_if_handles_are_equal() {
    let key = key(1, 1);
    for member in [
        Member::Root,
        Member::CommitMarker,
        Member::Resource(ResourceSlot::new(0)),
    ] {
        let entries = vec![(Some(name(key, member)), 1), (Some(name(key, member)), 1)];
        let error = discover(&entries).unwrap_err();
        assert_eq!(
            error,
            CatalogError::Duplicate {
                address: Address { key, member },
                first_input: 0,
                second_input: 1
            }
        );
    }
    // Nonunique unrelated manager tags are preserved, not silently treated as IDs.
    assert_eq!(
        discover(&[(Some("listener".into()), 1), (Some("listener".into()), 2)])
            .unwrap()
            .unrelated()
            .len(),
        2
    );
}
#[test]
fn missing_root_marker_and_slot_inventory_have_distinct_meanings() {
    let key = key(1, 1);
    let logical = manifest(&["state"]);
    let entries = vec![(Some(name(key, Member::Resource(ResourceSlot::new(0)))), 10)];
    let catalog = discover(&entries).unwrap();
    assert!(matches!(
        catalog.candidate(key).unwrap().map_manifest(&logical),
        Err(MappingError::MissingRoot)
    ));
    let entries = vec![
        (Some(name(key, Member::Root)), 1),
        (Some(name(key, Member::Resource(ResourceSlot::new(0)))), 10),
    ];
    let catalog = discover(&entries).unwrap();
    let candidate = catalog.candidate(key).unwrap();
    let prepared_mapping = candidate.map_manifest(&logical).unwrap();
    assert!(prepared_mapping.commit_marker.is_none()); // Still no execution authority.
    assert_eq!(
        candidate.require_marker().unwrap_err(),
        MappingError::MissingMarker
    );
    let entries = vec![
        (Some(name(key, Member::Root)), 1),
        (Some(name(key, Member::CommitMarker)), 2),
    ];
    let catalog = discover(&entries).unwrap();
    assert!(matches!(
        catalog.candidate(key).unwrap().map_manifest(&logical),
        Err(MappingError::ResourceCount {
            expected: 1,
            actual: 0
        })
    ));
    // Sparse/extreme indices never allocate by their numeric value.
    let entries = vec![
        (Some(name(key, Member::Root)), 1),
        (
            Some(name(key, Member::Resource(ResourceSlot::new(u32::MAX)))),
            10,
        ),
    ];
    let catalog = discover(&entries).unwrap();
    let candidate = catalog.candidate(key).unwrap();
    assert_eq!(candidate.resources().len(), 1);
    assert!(
        matches!(candidate.map_manifest(&logical), Err(MappingError::MissingSlot(slot)) if slot.index() == 0)
    );
}
#[test]
fn shared_resource_ids_bind_alias_roles_once_not_by_inode_or_name_guessing() {
    let key = key(1, 1);
    let mut logical = manifest(&["shared-output"]);
    logical.bundles[0].members.push(LogicalMember {
        id: MemberId::new("stderr").unwrap(),
        resource: logical.resources[0].clone(),
        extensions: vec![],
    });
    let entries = vec![
        (Some(name(key, Member::Root)), b"root".as_slice()),
        (
            Some(name(key, Member::Resource(ResourceSlot::new(0)))),
            b"shared image".as_slice(),
        ),
    ];
    let catalog = discover(&entries).unwrap();
    let mapping = catalog
        .candidate(key)
        .unwrap()
        .map_manifest(&logical)
        .unwrap();
    assert_eq!(mapping.resources.len(), 1);
    assert_eq!(
        logical.bundles[0].members[0].resource,
        logical.bundles[0].members[1].resource
    );
    assert_eq!(*mapping.resources[0].handle, b"shared image");
    logical.resources.push(logical.resources[0].clone());
    assert!(matches!(
        catalog.candidate(key).unwrap().map_manifest(&logical),
        Err(MappingError::Manifest(_))
    ));
}
#[test]
fn discovery_errors_and_successful_catalog_drop_do_not_close_original_handles() {
    struct Owner(Rc<Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let dropped = Rc::new(Cell::new(0));
    let key = key(1, 1);
    let entries = vec![
        (Some(name(key, Member::Root)), Owner(dropped.clone())),
        (Some("listener".into()), Owner(dropped.clone())),
    ];
    drop(discover(&entries).unwrap());
    assert_eq!(dropped.get(), 0);
    let result = Catalog::discover(
        entries.iter().map(|(name, handle)| NamedHandle {
            name: name.as_deref(),
            handle,
        }),
        Limits {
            max_entries: 1,
            max_generations: 1,
        },
    );
    assert!(matches!(result, Err(CatalogError::EntryLimit)));
    assert_eq!(dropped.get(), 0);
    let duplicate = Catalog::discover(
        [
            NamedHandle {
                name: entries[0].0.as_deref(),
                handle: &entries[0].1,
            },
            NamedHandle {
                name: entries[0].0.as_deref(),
                handle: &entries[1].1,
            },
        ],
        Limits::default(),
    );
    assert!(matches!(duplicate, Err(CatalogError::Duplicate { .. })));
    assert_eq!(dropped.get(), 0);
    drop(entries);
    assert_eq!(dropped.get(), 2);
}

#[test]
fn extra_slots_and_oversized_manifest_node_sets_are_not_valid_bindings() {
    let key = key(1, 1);
    let entries = vec![
        (Some(name(key, Member::Root)), 1),
        (Some(name(key, Member::Resource(ResourceSlot::new(0)))), 10),
        (Some(name(key, Member::Resource(ResourceSlot::new(1)))), 11),
    ];
    let catalog = discover(&entries).unwrap();
    let candidate = catalog.candidate(key).unwrap();
    assert!(matches!(
        candidate.map_manifest(&manifest(&["single"])),
        Err(MappingError::ResourceCount {
            expected: 1,
            actual: 2
        })
    ));
    let mut logical = manifest(&[]);
    logical.bundles = (0..MAX_METADATA_NODES + 1)
        .map(|_| Bundle {
            id: BundleId::new("unused").unwrap(),
            members: vec![],
            extensions: vec![],
        })
        .collect();
    assert!(matches!(
        candidate.map_manifest(&logical),
        Err(MappingError::MetadataLimit)
    ));
}
#[test]
fn bounded_scans_and_malformed_recognized_names_do_not_amplify_allocations() {
    let first = key(1, 1);
    let entries = [
        (Some(name(first, Member::Root)), 1),
        (Some(name(key(1, 2), Member::Root)), 2),
    ];
    assert!(matches!(
        Catalog::discover(
            entries.iter().map(|(name, handle)| NamedHandle {
                name: name.as_deref(),
                handle
            }),
            Limits {
                max_entries: 8,
                max_generations: 1
            }
        ),
        Err(CatalogError::GenerationLimit)
    ));
    assert!(matches!(
        Catalog::discover(
            std::iter::empty::<NamedHandle<'_, u32>>(),
            Limits {
                max_entries: MAX_ENTRIES + 1,
                ..Limits::default()
            }
        ),
        Err(CatalogError::InvalidLimits)
    ));
    let invalid = vec![(Some("ho1.malformed".into()), 1)];
    assert!(matches!(
        discover(&invalid),
        Err(CatalogError::MalformedName { input_index: 0, .. })
    ));
    let long_unrelated = "unrelated".repeat(100_000);
    let entries = vec![(Some(long_unrelated), 1)];
    let catalog = discover(&entries).unwrap();
    assert!(std::ptr::eq(
        catalog.unrelated()[0].name.unwrap().as_ptr(),
        entries[0].0.as_ref().unwrap().as_ptr()
    ));
}
