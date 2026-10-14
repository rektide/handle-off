use handle_off_handover::*;
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

fn generation() -> GenerationId {
    GenerationId::new("generation-1").unwrap()
}
fn evidence(value: &'static str) -> Evidence<&'static str> {
    Evidence::new(generation(), value)
}
#[derive(Default)]
struct Gates {
    old_running: bool,
    old_retired: bool,
    new_running: bool,
    new_holds_copies: bool,
}
impl Gates {
    fn assert_single_owner(&self) {
        assert!(!(self.old_running && self.new_running));
    }
    fn resume_old(&mut self, permit: ResumePermit) {
        assert_eq!(permit.generation(), &generation());
        assert!(!self.old_retired && !self.new_running && !self.new_holds_copies);
        self.old_running = true;
        self.assert_single_owner();
    }
    fn release_new(&mut self, grant: ReleaseGrant) {
        assert_eq!(grant.generation(), &generation());
        assert!(self.old_retired && !self.old_running && self.new_holds_copies);
        self.new_running = true;
        self.assert_single_owner();
    }
}
fn stage(attempt: &mut Attempt<'_, &'static str>, gates: &mut Gates) {
    attempt
        .publish(evidence("possible publication plan"), || {
            gates.new_holds_copies = true;
            Ok(evidence("delivery ACK only"))
        })
        .unwrap();
    assert_eq!(attempt.status().custody, Custody::Exposed);
    attempt
        .verify_custody(evidence("validate complete custody/dormancy"), || {
            assert!(gates.new_holds_copies && !gates.new_running && !gates.old_running);
            Ok(evidence("generation-scoped dormant custody verified"))
        })
        .unwrap();
}

#[test]
fn failed_publication_and_ambiguous_retirement_do_not_resume_old_owner() {
    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    {
        let mut attempt = controller.attempt().unwrap();
        let error = attempt
            .publish(evidence("journal ALL possible names before send"), || {
                gates.new_holds_copies = true;
                Err(evidence("partial delivery, ACK lost"))
            })
            .unwrap_err();
        assert!(matches!(error, TransitionError::ActionFailed { .. }));
        assert_eq!(attempt.status().custody, Custody::Exposed);
        assert!(
            attempt
                .abort(evidence("retire all copies"), || Err(evidence(
                    "retirement uncertain"
                )))
                .is_err()
        );
        assert!(!gates.old_running && !gates.new_running);
        gates.assert_single_owner();
    }
    assert!(controller.status().quarantined);
    assert!(
        controller
            .records()
            .iter()
            .any(|record| record.evidence.value == "partial delivery, ACK lost")
    );
    assert!(
        controller
            .records()
            .iter()
            .any(|record| record.evidence.value == "retirement uncertain")
    );
    let permit = controller
        .attempt()
        .unwrap()
        .abort(evidence("explicit recovery retirement"), || {
            gates.new_holds_copies = false;
            Ok(evidence("ALL copies and restoration opportunities retired"))
        })
        .unwrap();
    gates.resume_old(permit);
    assert_eq!(controller.status().custody, Custody::Retired);
    assert_eq!(controller.status().authority, Authority::OldResumeIssued);
    assert!(matches!(
        controller.attempt(),
        Err(TransitionError::Terminal)
    ));
}

#[test]
fn dropped_attempt_keeps_originals_evidence_and_quarantine_in_long_lived_controller() {
    struct Original(Rc<Cell<usize>>);
    impl Drop for Original {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let original = Original(drops.clone());
    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    {
        let mut attempt = controller.attempt().unwrap();
        stage(&mut attempt, &mut gates);
    }
    assert_eq!(drops.get(), 0);
    assert!(controller.status().quarantined);
    assert_eq!(controller.status().authority, Authority::ParkedOld);
    assert_eq!(
        controller.last_abandonment().unwrap().custody,
        Custody::Verified
    );
    assert_eq!(controller.abandoned_attempts(), 1);
    assert!(
        controller
            .records()
            .iter()
            .any(|record| record.evidence.value == "generation-scoped dormant custody verified")
    );
    {
        let attempt = controller.attempt().unwrap();
        assert!(attempt.status().quarantined);
    }
    assert!(!gates.old_running && !gates.new_running);
    assert_eq!(controller.abandoned_attempts(), 2);
    drop(original);
    assert_eq!(drops.get(), 1); // Explicit application disposal, not attempt Drop.
}

#[test]
fn irreversible_relinquishment_precedes_new_permission_even_when_release_fails() {
    for new_started in [false, true] {
        let mut controller = ParkingController::new_parked(generation());
        let mut gates = Gates::default();
        {
            let mut attempt = controller.attempt().unwrap();
            stage(&mut attempt, &mut gates);
            assert_eq!(
                attempt
                    .relinquish(evidence("disable old execution permanently"), || {
                        gates.old_retired = true;
                        RelinquishOutcome::Relinquished(evidence("old authority surrendered"))
                    })
                    .unwrap(),
                Relinquishment::Relinquished
            );
            let error = attempt
                .release(evidence("release control transaction"), |grant| {
                    assert!(gates.old_retired);
                    if new_started {
                        gates.release_new(grant);
                    }
                    Err(evidence(
                        "release failed or ACK lost, cannot infer non-delivery",
                    ))
                })
                .unwrap_err();
            assert!(matches!(error, TransitionError::ActionFailed { .. }));
            assert_eq!(attempt.status().authority, Authority::NewReleaseIssued);
            let mut called = false;
            assert!(
                attempt
                    .abort(evidence("forbidden old resume"), || {
                        called = true;
                        Ok(evidence("must not run"))
                    })
                    .is_err()
            );
            assert!(!called);
            assert!(!gates.old_running);
            gates.assert_single_owner();
        }
        assert!(controller.status().quarantined);
        assert!(matches!(
            controller.attempt(),
            Err(TransitionError::Terminal)
        ));
        assert!(
            controller
                .records()
                .iter()
                .any(|record| record.evidence.value
                    == "release failed or ACK lost, cannot infer non-delivery")
        );
    }
}

#[test]
fn not_relinquished_and_indeterminate_are_different_permissions() {
    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    let mut attempt = controller.attempt().unwrap();
    stage(&mut attempt, &mut gates);
    assert_eq!(
        attempt
            .relinquish(evidence("application relinquishment attempt"), || {
                RelinquishOutcome::NotRelinquished(evidence(
                    "definitely still parked, no revocation",
                ))
            })
            .unwrap(),
        Relinquishment::NotRelinquished
    );
    assert_eq!(attempt.status().authority, Authority::ParkedOld);
    // No resume approval until all successor custody is actually retired.
    assert!(
        attempt
            .abort(evidence("retire successor"), || Err(evidence(
                "unknown cleanup"
            )))
            .is_err()
    );
    assert!(!gates.old_running);
    gates.assert_single_owner();

    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    let mut attempt = controller.attempt().unwrap();
    stage(&mut attempt, &mut gates);
    assert_eq!(
        attempt
            .relinquish(evidence("relinquishment may have side effects"), || {
                gates.old_retired = true;
                RelinquishOutcome::Indeterminate(evidence(
                    "ordinary callback Err is not a no-effect proof",
                ))
            })
            .unwrap(),
        Relinquishment::Indeterminate
    );
    assert!(
        attempt
            .abort(evidence("cannot return old approval"), || Ok(evidence(
                "no copies"
            )))
            .is_err()
    );
    assert!(
        attempt
            .release(evidence("cannot release while uncertain"), |_| Ok(
                evidence("forbidden")
            ))
            .is_err()
    );
    attempt
        .verify_custody(evidence("fresh custody observation"), || {
            Ok(evidence("peer still dormant"))
        })
        .unwrap();
    assert!(attempt.status().quarantined); // Custody does not resolve old authority.
    attempt
        .confirm_relinquished(evidence(
            "application definitively confirms permanent surrender",
        ))
        .unwrap();
    attempt
        .release(evidence("explicit release after resolution"), |grant| {
            gates.release_new(grant);
            Ok(evidence("release acknowledged"))
        })
        .unwrap();
    gates.assert_single_owner();
}

#[test]
fn panics_leave_conservative_facts_and_complete_possible_publication_plan() {
    let mut controller = ParkingController::new_parked(generation());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            controller
                .attempt()
                .unwrap()
                .publish(
                    evidence("all planned names retained before callback"),
                    || panic!("publication callback panicked"),
                )
                .unwrap();
        }))
        .is_err()
    );
    assert_eq!(controller.status().custody, Custody::Exposed);
    assert!(controller.status().quarantined);
    assert_eq!(
        controller.records()[0].evidence.value,
        "all planned names retained before callback"
    );

    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let mut attempt = controller.attempt().unwrap();
            stage(&mut attempt, &mut gates);
            attempt
                .relinquish(evidence("relinquishment started before callback"), || {
                    gates.old_retired = true;
                    panic!("unknown outcome")
                })
                .unwrap();
        }))
        .is_err()
    );
    assert_eq!(controller.status().authority, Authority::Indeterminate);
    assert!(controller.status().quarantined);
    assert!(!gates.old_running && !gates.new_running);
}

#[test]
fn wrong_generation_evidence_is_retained_but_never_approves_a_permission() {
    let mut controller = ParkingController::new_parked(generation());
    let mut attempt = controller.attempt().unwrap();
    let other = || {
        Evidence::new(
            GenerationId::new("other").unwrap(),
            "wrong generation retained",
        )
    };
    let mut called = false;
    assert!(matches!(
        attempt.publish(other(), || {
            called = true;
            Ok(evidence("no"))
        }),
        Err(TransitionError::WrongGeneration { .. })
    ));
    assert!(!called);
    assert!(matches!(
        attempt.publish(evidence("correct start"), || Ok(other())),
        Err(TransitionError::WrongGeneration { .. })
    ));
    assert_eq!(attempt.status().custody, Custody::Exposed);
    assert!(
        attempt
            .verify_custody(evidence("verify start"), || Ok(other()))
            .is_err()
    );
    assert!(
        attempt
            .relinquish(evidence("forbidden unverified commit"), || {
                RelinquishOutcome::Relinquished(evidence("must not run"))
            })
            .is_err()
    );
    assert_eq!(attempt.status().authority, Authority::ParkedOld);
    assert!(
        attempt
            .abort(evidence("retirement start"), || Ok(other()))
            .is_err()
    );
    assert_eq!(attempt.status().authority, Authority::ParkedOld);
    assert!(
        attempt
            .records()
            .iter()
            .any(|record| record.evidence.generation.as_str() == "other")
    );
}

#[test]
fn linear_approvals_have_one_issuance_and_do_not_act_on_drop() {
    let mut controller = ParkingController::new_parked(generation());
    let mut gates = Gates::default();
    {
        let mut attempt = controller.attempt().unwrap();
        let permit = attempt
            .abort(evidence("local-only explicit abort"), || {
                Ok(evidence("no external restoration opportunity"))
            })
            .unwrap();
        drop(permit); // No automatic application resume.
        assert!(!gates.old_running);
        assert!(
            attempt
                .abort(evidence("second permit forbidden"), || Ok(evidence(
                    "must not run"
                )))
                .is_err()
        );
    }
    assert!(!controller.status().quarantined);
    assert!(matches!(
        controller.attempt(),
        Err(TransitionError::Terminal)
    ));

    let mut controller = ParkingController::new_parked(generation());
    let mut attempt = controller.attempt().unwrap();
    stage(&mut attempt, &mut gates);
    attempt
        .relinquish(evidence("retire old"), || {
            gates.old_retired = true;
            RelinquishOutcome::Relinquished(evidence("retired"))
        })
        .unwrap();
    attempt
        .release(evidence("grant may be dropped by backend"), |grant| {
            drop(grant);
            Ok(evidence("backend chose not to start execution"))
        })
        .unwrap();
    assert!(!gates.old_running && !gates.new_running);
    assert!(
        attempt
            .release(evidence("no second grant"), |_| Ok(evidence(
                "must not run"
            )))
            .is_err()
    );
}
