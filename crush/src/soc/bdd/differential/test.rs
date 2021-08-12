use super::*;

#[test]
fn simple_test_depfinder_iter() {
    let simple = crate::bdd!(5;0;
            [
            ("1+2",[(1;2,3)]);
            ("3+2",[(2;4,5);(3;4,0)]);
            ("0+4",[(4;0,6);(5;6,0)]);
            ("",[(6;0,0)])
            ]);

    //Node Id's are re-named when imported through the macro(?). Updated below accordingly.
    let deps = DepBoolFinder::new(Id::new(10000),
                                  0,
                                  NonZeroUsize::new(2).unwrap(),
                                  &simple);

    let mut d_iter =  deps.iter();

    assert_eq!(d_iter.next(), Some((Id::new(40000), false)).as_ref());
    assert_eq!(d_iter.next(), Some((Id::new(50000), true)).as_ref());
    assert_eq!(d_iter.next(), Some((Id::new(40000), true)).as_ref());
    assert_eq!(d_iter.next(), None);
}


#[test]
fn simple_test_sbox_count() {
    // First and second assert checks full range, for step 3 and 2.
    // Third and onwards tests various ranges/offsets
    let simple = crate::bdd!(6;1;
            [
            ("0",[(1;2,3)]);
            ("1",[(2;4,5);(3;6,7)]);
            ("2",[(4;8,9);(5;10,11);(6;11,12);(7;0,12)]);
            ("3",[(8;13,14);(9;14,0);(10;13,14);(11;0,15);(12;15,0)]);
            ("4",[(13;16,0);(14;0,16);(15;0,17)]);
            ("5",[(16;18,0);(17;0,18)]);
            ("",[(18;0,0)])
            ]);

    let arena = simple.identify_trails_and_weights(.., 3);

    let arena_e: BTreeMap<usize,
        HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>>> =
        [(3,
          [(Id::new(80001), 3),
              (Id::new(90001), 2),(Id::new(100001), 3),
              (Id::new(110001), 2),(Id::new(120001), 2)
          ].iter().cloned().collect()),
            (0,
             [(Id::new(10001), 7)
             ].iter().cloned().collect())
        ].iter().cloned().collect();
    let expected = NWArena{top: 0, bottom: 6, arena: arena_e, lsb_map: Default::default() };

    assert_eq!(expected, arena);
    println!("Passed first assert.");

    let arena = simple.identify_trails_and_weights(.., 2);
    let arena_e: BTreeMap<usize,
        HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>>> =
        [(0,
          [(Id::new(10001), 15)
          ].iter().cloned().collect()),
            (2,
             [(Id::new(40001), 5),(Id::new(50001), 5),
                 (Id::new(60001), 4), (Id::new(70001), 4)
             ].iter().cloned().collect()),
            (4,
             [
                 (Id::new(130001), 1), (Id::new(140001), 2), (Id::new(150001), 2)
             ].iter().cloned().collect()),
        ].iter().cloned().collect();
    let expected = NWArena{top: 0, bottom: 6, arena: arena_e, lsb_map: Default::default() };

    assert_eq!(arena, expected);
    println!("Passed second assert!");

    let arena = simple.identify_trails_and_weights(..=5, 2);
    assert_eq!(arena, expected);
    println!("Passed third assert!");

    let arena = simple.identify_trails_and_weights(0..=5, 2);
    assert_eq!(arena, expected);
    println!("Passed fourth assert!");

    let arena = simple.identify_trails_and_weights(0..6, 2);
    assert_eq!(arena, expected);
    println!("Passed fifth assert!");

    let arena = simple.identify_trails_and_weights(0.., 2);
    assert_eq!(arena, expected);
    println!("Passed sixth assert!");

    //Testing offsets from top and bottom:

    let arena_e: BTreeMap<usize,
        HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>>> =
        [(2,
          [(Id::new(40001), 5),(Id::new(50001), 5),
              (Id::new(60001), 4), (Id::new(70001), 4)
          ].iter().cloned().collect()),
            (4,
             [
                 (Id::new(130001), 1), (Id::new(140001), 2), (Id::new(150001), 2)
             ].iter().cloned().collect()),
        ].iter().cloned().collect();
    let expected = NWArena{top: 2, bottom: 6, arena: arena_e, lsb_map: Default::default() };

    let arena = simple.identify_trails_and_weights(2.., 2);
    assert_eq!(arena, expected);
    println!("Passed seventh assert!");

    // Testing offset, but also that we only have "one step" then return
    let arena_e: BTreeMap<usize,
        HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>>> =
        [(2,
          [(Id::new(40001), 3),(Id::new(50001), 3),
              (Id::new(60001), 2), (Id::new(70001), 2)
          ].iter().cloned().collect()),
        ].iter().cloned().collect();
    let expected = NWArena{top: 2, bottom: 4, arena: arena_e, lsb_map: Default::default() };

    let arena = simple.identify_trails_and_weights(2..4, 2);
    assert_eq!(arena, expected);
    println!("Passed eight assert!");

}

#[ignore]
#[test]
fn test_prune_simple() {
    todo!("Rework to make compile again, after much modification in paren module");
    // let mut actual = crate::bdd!(6;1;
    //         [
    //         ("0",[(1;2,3)]);
    //         ("1",[(2;4,5);(3;6,7)]);
    //         ("2",[(4;8,0);(5;0,8);(6;9,0);(7;10,0)]);
    //         ("3",[(8;11,0);(9;12,0);(10;0,12)]);
    //         ("4",[(11;13,0);(12;0,14)]);
    //         ("5",[(13;15,0);(14;0,15)]);
    //         ("",[(15;0,0)])
    //         ]);
    //
    // println!("Before: \n{:#?}", actual);
    // let deleted = actual.weight_based_prune(2, .., 2);
    // println!("After : \n{:#?}", actual);
    //
    // let arena_e: BTreeMap<usize,
    //     HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>>> =
    //     [
    //         (2,
    //          [(Id::new(70001), 4),
    //          ].iter().cloned().collect()),
    //     ].iter().cloned().collect();
    // let expected_deleted = NWArena{top: 0, bottom: 6, arena: arena_e, lsb_map: Default::default() };
    // let expected = crate::bdd!(6;1;
    //         [
    //         ("0",[(1;2,3)]);
    //         ("1",[(2;4,5);(3;6,0)]);
    //         ("2",[(4;8,0);(5;0,8);(6;9,0)]);
    //         ("3",[(8;11,0);(9;12,0)]);
    //         ("4",[(11;13,0);(12;0,14)]);
    //         ("5",[(13;15,0);(14;0,15)]);
    //         ("",[(15;0,0)])
    //         ]);
    //
    // assert_eq!(deleted, expected_deleted);
    // println!("Passed first assert!");
    // assert_eq!(actual, expected);
    //
    // println!("'Actual' after second: \n{:#?}", actual);

    // panic!("Forced");

}

struct EmptyLibrarian {

}

impl PruneLogger for EmptyLibrarian {
    fn record(&mut self, rec: PruneRecord) {
    }
}
