//! A module that contains the logic for the 2048 game.

// std imports
use std::fmt::{self, Display, Formatter, Write};
use std::sync::{Arc, Mutex};

// external imports
use rand::seq::IteratorRandom;
use rand::{random, thread_rng};
use tinypool::ThreadPool;

// internal imports
use crate::error::Error;

/// An enum that represents the moves that can be made in the game of 2048.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GameMove {
    Left,
    Right,
    Up,
    Down,
}
impl GameMove {
    /// Returns the index of the move.
    /// Used internally for indexing arrays.
    /// # Returns
    /// * ```usize``` - The index of the move.
    fn index(&self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Up => 2,
            Self::Down => 3,
        }
    }

    /// Returns the move from the index.
    /// Used internally for indexing arrays.
    /// # Arguments
    /// * ```index``` - The index of the move.
    /// # Returns
    /// * ```GameMove``` - The move.
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Left,
            1 => Self::Right,
            2 => Self::Up,
            3 => Self::Down,
            _ => panic!("Invalid index: {}", index),
        }
    }
}

/// An enum that represents the possible states of the 2048 game.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GameState {
    /// The game is in progress.
    InProgress,
    /// The game is over. Result is either victory or loss.
    GameOver,
}

/// An enum that represents the possible results of the 2048 game.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GameResult {
    /// The result is not yet determined.
    Pending,
    /// The game is won, 2048 was reached.
    Victory,
    /// The game is over, 2048 was not reached, there are no valid moves left.
    Loss,
}

#[derive(Debug)]
/// A struct that represents the 2048 game.
pub struct Game<const SIZE: usize> {
    /// Game tiles.
    board: [[u64; SIZE]; SIZE],
    /// Game score.
    score: u64,
    /// Additional score for each move.
    score_next: [u64; 4],
    /// Availability of moves.
    moves: [bool; 4],
    /// Board after each of the moves.
    moves_next: [[[u64; SIZE]; SIZE]; 4],
    /// The state of the game.
    state: GameState,
    /// The result of the game.
    result: GameResult,
}
impl<const SIZE: usize> Game<SIZE> {
    /// Creates a new game of 2048.
    /// # Returns
    /// * ```Ok(Game)```: The game was created successfully.
    /// * ```Err(Error)```: The game was not created successfully.
    /// # Errors
    /// * ```Error::InvalidSize```: The SIZE is invalid. Must be at least 4.
    pub fn new() -> Result<Self, Error> {
        if SIZE < 4 {
            return Err(Error::InvalidSize);
        }

        let board = [[0; SIZE]; SIZE];
        let score = 0;
        let score_next = [0; 4];
        let moves = [true; 4];
        let moves_next = [[[0; SIZE]; SIZE]; 4];
        let state = GameState::InProgress;
        let result = GameResult::Pending;

        let mut game_object: Self = Self {
            board,
            score,
            score_next,
            moves,
            moves_next,
            state,
            result,
        };

        game_object.new_tile();
        game_object.update();

        Ok(game_object)
    }

    /// Creates a game of 2048 from an existing board.
    /// The board must be a square matrix filled with 0 for empty tiles and powers of 2 for filled tiles.
    /// # Arguments
    /// * ```board```: The board to use.
    /// * ```score```: The score of the game.
    /// # Returns
    /// * ```Ok(Game)```: The game was created successfully.
    /// * ```Err(Error)```: The game was not created successfully.
    /// # Errors
    /// * ```Error::InvalidSize```: The SIZE is invalid. Must be at least 4.
    /// * ```Error::InvalidValue```: The board contains invalid value. Must be 0 or a power of 2, starting from 2.
    pub fn from_existing(board: &[[u64; SIZE]; SIZE], score: u64) -> Result<Self, Error> {
        if SIZE < 4 {
            return Err(Error::InvalidSize);
        }

        for row in board.iter() {
            for tile in row.iter() {
                if *tile == 1 || (*tile != 0 && !tile.is_power_of_two()) {
                    return Err(Error::InvalidValue);
                }
            }
        }

        let board = *board;
        let score_next = [0; 4];
        let moves = [true; 4];
        let moves_next = [[[0; SIZE]; SIZE]; 4];
        let state = GameState::InProgress;
        let result = GameResult::Pending;

        let mut game_object = Self {
            board,
            score,
            score_next,
            moves,
            moves_next,
            state,
            result,
        };
        game_object.update();

        Ok(game_object)
    }

    /// Returns the reference to the board.
    /// The board is a square matrix filled with 0 for empty tiles and powers of 2 for filled tiles.
    /// # Returns
    /// * ```&[[u64; SIZE]; SIZE]```: The board.
    pub fn board(&self) -> &[[u64; SIZE]; SIZE] {
        &self.board
    }

    /// Returns the result of the game.
    /// # Returns
    /// * ```Result::Victory```: The game is won, 2048 was reached.
    /// * ```Result::Pending```: The game is in progress, 2048 is not reached yet.
    /// * ```Result::Loss```: The game is over, 2048 was not reached.
    pub fn result(&self) -> GameResult {
        self.result
    }

    /// Returns the score of the game.
    /// # Returns
    /// * ```u64```: The score of the game.
    pub fn score(&self) -> u64 {
        self.score
    }

    /// Returns the size of the board.
    /// # Returns
    /// * ```usize```: The size of the board. The board is ```usize```x```usize```.
    pub fn size(&self) -> usize {
        SIZE
    }

    /// Returns the state of the game.
    /// # Returns
    /// * ```State::InProgress```: The game is in progress.
    /// * ```State::GameOver```: The game is over.
    pub fn state(&self) -> GameState {
        self.state
    }

    /// Make a move in the game.
    /// # Arguments
    /// * ```direction```: The direction to move in.
    /// # Returns
    /// * ```true``` - The move was successful.
    /// * ```false``` - The move was invalid/impossible.
    pub fn make_move(&mut self, direction: GameMove) -> bool {
        let next_ind = direction.index();
        if self.moves[next_ind] {
            self.board = self.moves_next[next_ind];
            self.score += self.score_next[next_ind];
            self.new_tile();
            self.update();
            true
        } else {
            false
        }
    }

    /// Add a new tile to the board.
    fn new_tile(&mut self) {
        // create iterator over all tiles (cartesian product of two ranges)
        // filter only empty tiles -> get iterator over empty tiles
        // choose one of the empty tiles with rng
        let loc = (0..SIZE)
            .flat_map(|ind1| (0..SIZE).map(move |ind2| (ind1, ind2)))
            .filter(|&pos| self.board[pos.0][pos.1] == 0)
            .choose(&mut thread_rng())
            .unwrap();

        // add 2 or 4 to that tile
        self.board[loc.0][loc.1] = if random::<f64>() < 0.9 { 2 } else { 4 };
    }

    /// Update moves, moves_next, score_next, state and result.
    fn update(&mut self) {
        // update left
        self.score_next[0] = 0;
        for (i, row) in self.board.iter().enumerate() {
            let mut j = 0;
            let mut merge = false;
            for elem in row.iter().filter(|&&x| x != 0) {
                if merge && *elem == self.moves_next[0][i][j - 1] {
                    self.moves_next[0][i][j - 1] *= 2;
                    self.score_next[0] += self.moves_next[0][i][j - 1];
                    merge = false;
                } else {
                    self.moves_next[0][i][j] = *elem;
                    j += 1;
                    merge = true;
                }
            }
            for empty_elem in self.moves_next[0][i].iter_mut().skip(j) {
                *empty_elem = 0;
            }
        }
        self.moves[0] = self.board != self.moves_next[0];

        // update right
        self.score_next[1] = 0;
        for (i, row) in self.board.iter().enumerate() {
            let mut j = SIZE - 1;
            let mut merge = false;
            let mut negative_index = false;
            for elem in row.iter().filter(|&&x| x != 0).rev() {
                if merge && *elem == self.moves_next[1][i][j + 1] {
                    self.moves_next[1][i][j + 1] *= 2;
                    self.score_next[1] += self.moves_next[1][i][j + 1];
                    merge = false;
                } else {
                    self.moves_next[1][i][j] = *elem;
                    j = match j.checked_sub(1) {
                        Some(x) => x,
                        None => {
                            // we processed the whole row, we can safely break
                            negative_index = true;
                            break;
                        }
                    };
                    merge = true;
                }
            }
            if !negative_index {
                for empty_elem in self.moves_next[1][i].iter_mut().rev().skip(SIZE - 1 - j) {
                    *empty_elem = 0;
                }
            }
        }
        self.moves[1] = self.board != self.moves_next[1];

        // update up
        self.score_next[2] = 0;
        for col in 0..SIZE {
            let mut i = 0;
            let mut merge = false;
            for elem in self.board.iter().map(|row| row[col]).filter(|&x| x != 0) {
                if merge && elem == self.moves_next[2][i - 1][col] {
                    self.moves_next[2][i - 1][col] *= 2;
                    self.score_next[2] += self.moves_next[2][i - 1][col];
                    merge = false;
                } else {
                    self.moves_next[2][i][col] = elem;
                    i += 1;
                    merge = true;
                }
            }
            for empty_elem in self.moves_next[2].iter_mut().skip(i).map(|row| &mut row[col]) {
                *empty_elem = 0;
            }
        }
        self.moves[2] = self.board != self.moves_next[2];

        // update down
        self.score_next[3] = 0;
        for col in 0..SIZE {
            let mut i = SIZE - 1;
            let mut merge = false;
            let mut negative_index = false;
            for elem in self.board.iter().map(|row| row[col]).filter(|&x| x != 0).rev() {
                if merge && elem == self.moves_next[3][i + 1][col] {
                    self.moves_next[3][i + 1][col] *= 2;
                    self.score_next[3] += self.moves_next[3][i + 1][col];
                    merge = false;
                } else {
                    self.moves_next[3][i][col] = elem;
                    i = match i.checked_sub(1) {
                        Some(x) => x,
                        None => {
                            // we processed whole column, we can safely break
                            negative_index = true;
                            break;
                        }
                    };
                    merge = true;
                }
            }
            if !negative_index {
                for empty_elem in self.moves_next[3].iter_mut().rev().skip(SIZE - 1 - i).map(|row| &mut row[col]) {
                    *empty_elem = 0;
                }
            }
        }
        self.moves[3] = self.board != self.moves_next[3];

        // update state
        if self.moves.iter().all(|&x| !x) {
            self.state = GameState::GameOver;
        }

        // update result
        match self.result {
            GameResult::Pending => {
                let victory = self.board.iter().flat_map(|row| row.iter()).any(|&x| x >= 2048);
                if victory {
                    self.result = GameResult::Victory;
                } else if self.state == GameState::GameOver {
                    self.result = GameResult::Loss;
                }
            }
            GameResult::Victory => {}
            GameResult::Loss => {}
        }
    }

    /// Find the best move to make based on the current board state.
    /// Based on Monte Carlo algorithm (randomized guessing).
    /// Uses multiple threads to speed up the process.
    /// # Arguments
    /// * ```depth``` - The number of simulated games to play to determine the best move. Recommended value is 1000.
    /// # Returns
    /// * ```Ok(GameMove)``` - The best move to make.
    /// * ```Err(Error)``` - There are no valid moves left.
    /// # Errors
    /// * ```Error::NoValidMove``` - There are no valid moves left.
    pub fn find_best_move(&self, depth: usize) -> Result<GameMove, Error> {
        let possible_moves_count = self.moves.iter().filter(|&&x| x).count();

        match possible_moves_count {
            0 => Err(Error::NoValidMove),
            1 => Ok(GameMove::from_index(self.moves.iter().position(|&val| val).unwrap())),
            2.. => {
                let mut thread_pool = ThreadPool::new(None).unwrap();

                let mut depth_per_thread = depth / (possible_moves_count * thread_pool.size());
                if depth_per_thread == 0 {
                    depth_per_thread = 1;
                } else if depth_per_thread * possible_moves_count * thread_pool.size() != depth {
                    depth_per_thread += 1;
                }

                let moves_values = Arc::new(Mutex::new([0; 4]));

                for move_ind in self.moves.iter().enumerate().filter_map(|(ind, &x)| if x { Some(ind) } else { None }) {
                    let move_type = GameMove::from_index(move_ind);

                    for _ in 0..thread_pool.size() {
                        let board_copy = self.board;
                        let moves_values = Arc::clone(&moves_values);
                        thread_pool.add_to_queue(move || {
                            let mut thread_score = 0;

                            for _ in 0..depth_per_thread {
                                let mut work_game = Self::from_existing(&board_copy, 0).unwrap();

                                work_game.make_move(move_type);
                                while let GameState::InProgress = work_game.state {
                                    if work_game.make_move(
                                        work_game
                                            .moves
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(i, &b)| if b { Some(GameMove::from_index(i)) } else { None })
                                            .choose(&mut thread_rng())
                                            .unwrap(),
                                    ) && work_game.state == GameState::GameOver
                                    {
                                        break;
                                    }
                                }

                                thread_score += work_game.score;
                            }

                            moves_values.lock().unwrap()[move_ind] += thread_score;
                        });
                    }
                }
                thread_pool.join();

                let max_ind = moves_values.lock().unwrap().iter().enumerate().max_by_key(|(_, &x)| x).unwrap().0;

                Ok(GameMove::from_index(max_ind))
            }
        }
    }
}
impl<const SIZE: usize> Display for Game<SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // find the maximum value in the board
        let mut max_val = 0;
        for row in &self.board {
            for val in row {
                if *val > max_val {
                    max_val = *val;
                }
            }
        }

        // find the number of digits in the maximum value
        let mut max_len = 0;
        if max_val == 0 {
            max_len = 1;
        }
        while max_val != 0 {
            max_len += 1;
            max_val /= 10;
        }
        max_len += 1; // add one space

        // create the output string
        let mut output = String::from("Board:\n");
        for row in &self.board {
            for val in row {
                write!(&mut output, "{:width$}", val, width = max_len).unwrap();
            }
            output.push('\n');
        }
        writeln!(&mut output, "Score: {}", self.score).unwrap();

        write!(f, "{}", output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_game_4() {
        //! Test the creation of a new game with the default size (4x4)

        let game: Game<4> = Game::new().unwrap();
        let game_from = Game::from_existing(game.board(), 0).unwrap();

        assert_eq!(game.size(), 4);
        assert_eq!(game_from.size(), 4);

        assert_eq!(game.score(), 0);
        assert_eq!(game_from.score(), 0);

        assert_eq!(game.state(), GameState::InProgress);
        assert_eq!(game_from.state(), GameState::InProgress);

        assert_eq!(game.result(), GameResult::Pending);
        assert_eq!(game_from.result(), GameResult::Pending);
    }

    #[test]
    fn create_game_5() {
        //! Test the creation of a new game with a bigger size (5x5)

        let game: Game<5> = Game::new().unwrap();
        let game_from = Game::from_existing(game.board(), 0).unwrap();

        assert_eq!(game.size(), 5);
        assert_eq!(game_from.size(), 5);

        assert_eq!(game.score(), 0);
        assert_eq!(game_from.score(), 0);

        assert_eq!(game.state(), GameState::InProgress);
        assert_eq!(game_from.state(), GameState::InProgress);

        assert_eq!(game.result(), GameResult::Pending);
        assert_eq!(game_from.result(), GameResult::Pending);
    }

    #[test]
    fn game_4_ai() {
        //! Test the AI's ability to play a game with the default size (4x4)

        let mut game: Game<4> = Game::new().unwrap();
        while let Ok(best_move) = game.find_best_move(1_000) {
            game.make_move(best_move);
        }

        assert_eq!(game.state(), GameState::GameOver);
        assert_ne!(game.score(), 0);
    }

    #[test]
    fn game_5_ai() {
        //! Test the AI's ability to play a big game

        let mut game: Game<5> = Game::new().unwrap();
        while let Ok(best_move) = game.find_best_move(1_000) {
            game.make_move(best_move);
        }

        assert_eq!(game.state(), GameState::GameOver);
        assert_ne!(game.score(), 0);
    }
}
