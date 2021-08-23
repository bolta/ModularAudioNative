// マクロを提供するモジュール（common::parser）はマクロを使うモジュールより先に、
// かつ #[macro_use] をつけて宣言する必要がある
// https://stackoverflow.com/questions/26731243/how-do-i-use-a-macro-across-module-files
// #[macro_use]
// pub mod parser;

pub mod stack;
