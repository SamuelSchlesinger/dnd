# Dungeons & Dragons: AI Dungeon Master

```
  _____          _____                                                         
 |  __ \        |  __ \                                                        
 | |  | | _ __  | |  | |
 | |  | || '_ \ | |  | |
 | |__| || | | || |__| |
 |_____/ |_| |_||_____/
 
```

## About

Experience the magic of Dungeons & Dragons with an AI Dungeon Master! This immersive command-line game brings the theater-of-the-mind experience of D&D to life through rich storytelling, character creation, and interactive gameplay powered by GPT-4o. Create your character, embark on epic quests, and let your imagination guide your adventure.

## Features

- **Complete D&D 5e Character Creation**: Create characters with customizable races, classes, abilities, skills, and backgrounds
- **AI Dungeon Master**: Experience dynamic storytelling with an AI that crafts rich narratives and responds to your actions
- **Immersive World**: Explore detailed fantasy environments with evocative descriptions
- **Skill Checks & Dice Rolling**: Test your abilities with authentic D&D mechanics
- **Character Progression**: Level up and develop your character as you adventure
- **Colorful CLI Interface**: Enjoy a visually appealing terminal experience with colorful text, ASCII art, and intuitive UI
- **Auto-Save**: Your adventure progress is automatically saved so you can continue your journey later

## Installation

1. Ensure you have Rust installed ([install Rust](https://www.rust-lang.org/tools/install))
2. Clone this repository
3. Create a `.env` file with your OpenAI API key:
   ```
   OPENAI_API_KEY=your_api_key_here
   ```
5. Build and run the game:
   ```
   cargo build --release
   cargo run --release
   ```

## How to Play

1. Start a new adventure or continue a saved one
2. Create your character:
   - Choose race, class, and background
   - Determine ability scores (roll, standard array, or point buy)
   - Select skill proficiencies
   - Receive starting equipment
3. Begin your adventure with the AI Dungeon Master
4. Take actions by describing what your character does
5. Roll skill checks when attempting difficult tasks
6. Roll dice for combat and other game mechanics
7. Save your progress at any time

## Game Features

### Character Creation
The game supports D&D 5e character creation with:
- 9 playable races
- 13 character classes
- 13 character backgrounds
- Multiple ability score generation methods
- Class-appropriate skill proficiencies
- Starting equipment based on class

### AI Storytelling
The AI Dungeon Master creates:
- Engaging campaign hooks and quests
- Detailed descriptions of locations and NPCs
- Dynamic responses to player actions
- Appropriate challenges based on character abilities
- A consistent and immersive fantasy world

### Dice Mechanics
Authentic D&D dice mechanics include:
- Skill checks (d20 + ability modifier + proficiency)
- Variable dice types (d4, d6, d8, d10, d12, d20, d100)
- Multi-dice rolls (2d6, 3d8, etc.)
- Automatic calculation of modifiers and bonuses

## Requirements

- Rust 2024 Edition
- OpenAI API key (for GPT-4o access)
- Terminal with color support

## License

This project is open source and available under the MIT License.

---

*"Roll for initiative, adventurer!"*
