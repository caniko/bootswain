use bootswain_core::Board;

#[test]
fn pi5_and_rock5a_board_names_are_distinct_from_older_profiles() {
    for (name, alias) in [("raspberry-pi-5", "pi5"), ("rock-5a", "rock5a")] {
        let board = alias.parse::<Board>().unwrap();
        assert_eq!(board.as_str(), name);
        assert_eq!(
            serde_json::to_string(&board).unwrap(),
            format!("\"{name}\"")
        );
        assert_eq!(
            serde_json::from_str::<Board>(&format!("\"{name}\"")).unwrap(),
            board
        );
        assert_ne!(board, Board::RockPro64);
        assert_ne!(board, Board::RaspberryPi3BPlus);
    }
}
