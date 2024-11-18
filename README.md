# Feednotes

A simple todo list app written for myself.

The main purpose of this project was for me to learn
[ratatui](https://github.com/ratatui/ratatui),
[tui-widget-ist](https://github.com/preiter93/tui-widget-list), and
[tui-textarea](https://github.com/rhysd/tui-textarea)

Reads and save notes in `$HOME/.local/share/feednotes/notes.json`. `Feednotes`
will not create the file for you.

Only a small subset of vim keybindings in textarea are implemented. Experiment
it yourself.

## Controls

Feed view:

| key | function |
| - | - |
| `q` | quit |
| `j` | next note |
| `k` | previous note |
| `n` | new note (enters composer view) |
| `i` | edit note (enters composer view) |
| `/` | filtering mode (enters composer view in insert mode) |

Composer view (normal mode):

| key | function |
| - | - |
| `W` | save and exit composer view |
| `backspace` | exit composer view |
