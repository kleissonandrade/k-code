use std::path::PathBuf;

use crate::mode::EditorMode;

#[derive(Debug, Clone, PartialEq)]
pub enum CursorMotion {
    Up,
    Down,
    Left,
    Right,
    LineStart,
    LineEnd,
    WordForward,
    WordBackward,
    PageUp,
    PageDown,
    FileStart,
    FileEnd,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Tick,
    Render,
    Quit,
    Resize(u16, u16),

    EnterMode(EditorMode),
    ExitMode,

    NewFile,
    OpenFile(PathBuf),
    SaveFile,
    CloseBuffer,
    NextBuffer,
    PrevBuffer,

    MoveCursor(CursorMotion),
    InsertChar(char),
    InsertNewline,
    DeleteCharBackward,
    DeleteCharForward,
    DeleteLine,
    YankLine,
    Paste,
    Undo,
    Redo,

    ToggleFileTree,
    FileTreeUp,
    FileTreeDown,
    FileTreeSelect,
    FileTreeExpand,
    FileTreeCollapse,

    SearchInFile(String),
    SearchNext,
    SearchPrev,
    SearchClear,
    FuzzySearch(String),
    FuzzySelect,

    GoToDefinition,
    GoToLine(usize),

    GitCommit(String),
    GitPush,
    GitPull,
    GitCheckout(String),
    GitStash(String),
    GitStashPop,
    GitStageFile(String),
    GitStageAll,
    GitRefresh,

    SwitchTheme,
    ShowTutorial,

    CommandExecute(String),

    ToggleTerminal,
    NewTerminal,
    CloseTerminal,
    NextTerminalTab,
    PrevTerminalTab,

    StatusMessage(String),
    Error(String),
    Noop,
}
