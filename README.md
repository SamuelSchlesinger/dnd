# Mind Reader: 20 Questions Game

```
 __  __ _           _   ____                _           
|  \/  (_)_ __   __| | |  _ \ ___  __ _  __| | ___ _ __ 
| |\/| | | '_ \ / _` | | |_) / _ \/ _` |/ _` |/ _ \ '__|
| |  | | | | | | (_| | |  _ <  __/ (_| | (_| |  __/ |   
|_|  |_|_|_| |_|\__,_| |_| \_\___|\__,_|\__,_|\___|_|   
                                                        
```

## About

Mind Reader is an interactive command-line implementation of the classic 20 Questions game powered by GPT-4o. The Mind Reader thinks of something from a selected category, and you must guess what it is by asking up to 20 yes/no questions.

## Features

- **Beautiful CLI Interface**: Enjoy a visually appealing terminal experience with colorful text, progress spinners, and ASCII art
- **Three Categories**: Choose from Person, Place, or Thing
- **Dynamic AI-Generated Content**: The Mind Reader (GPT-4o) thinks of unique subjects for you to guess
- **20 Questions Format**: Ask up to 20 yes/no questions to narrow down the answer
- **Auto-Save**: Your game progress is automatically saved so you can continue your game later

## Installation

1. Ensure you have Rust installed ([install Rust](https://www.rust-lang.org/tools/install))
2. Clone this repository
3. Create a `.env` file with your OpenAI API key:
   ```
   OPENAI_API_KEY=your_api_key_here
   ```
4. Build and run the game:
   ```
   cargo build --release
   cargo run --release
   ```

## How to Play

1. Start a new game or continue a saved one
2. Choose a category: Person, Place, or Thing
3. Ask up to 20 yes/no questions to gather information
4. Make your final guess at any time by typing 'guess'
5. See if you've correctly identified what the Mind Reader was thinking!

### Special Commands

- Type `guess` to make your final guess about what the Mind Reader is thinking of

## Requirements

- Rust 2024 Edition
- OpenAI API key (for GPT-4o access)

## License

This project is open source and available under the MIT License.

---

*"Can you guess what I'm thinking?"*