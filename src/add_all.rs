use std::cmp::Ordering;

use anyhow::Result;

use crate::name_parser::NameParser;

const INDENT: &str = "    ";
const ALLWAYS_START_COMMENT: &str = "# allways: start";
const ALLWAYS_END_COMMENT: &str = "# allways: end";

pub fn do_it_allways(src: &str) -> Result<Option<String>> {
    let names = get_public_names(src)?;
    if names.is_empty() {
        return Ok(None);
    }
    let allways_string = get_allways_string(names);
    Ok(Some(match get_file_state(src) {
        FileState::NoAll => insert_new_allways_block(src, allways_string),
        FileState::YesAll(start, end) => update_allways_block(src, start, end, allways_string),
    }))
}

#[derive(PartialEq, Debug)]
enum FileState {
    NoAll,
    YesAll(usize, usize),
}

fn get_file_state(src: &str) -> FileState {
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;

    let mut offset = 0_usize;
    for line in src.lines() {
        match line.trim_end() {
            ALLWAYS_START_COMMENT => {
                start = Some(offset);
            }
            ALLWAYS_END_COMMENT => {
                end = Some(offset + line.len() + 1);
            }
            _ => {}
        }
        offset += line.len() + 1;
    }

    if let (Some(start), Some(end)) = (start, end) {
        if start < end {
            return FileState::YesAll(start, end);
        }
    }

    FileState::NoAll
}

fn get_allways_string(names: Vec<String>) -> String {
    let names_str = names
        .into_iter()
        .map(|name| format!("{INDENT}\"{name}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "\
{ALLWAYS_START_COMMENT}
__all__ = [
{names_str},
]
{ALLWAYS_END_COMMENT}
"
    )
}

fn insert_new_allways_block(src: &str, mut allways_string: String) -> String {
    allways_string.insert_str(0, "\n\n");
    allways_string.insert_str(0, src);
    allways_string
}

fn update_allways_block(src: &str, start: usize, end: usize, mut allways_string: String) -> String {
    allways_string.insert_str(0, &src[..start]);
    if end < src.len() {
        allways_string.push_str(&src[end..]);
    }
    allways_string
}

fn get_public_names(src: &str) -> Result<Vec<String>> {
    let mut public_names = src
        .parse::<NameParser>()?
        .into_iter()
        .filter(|s| !s.starts_with('_'))
        .collect::<Vec<_>>();
    public_names.sort_by(case_insensitive_cmp);
    Ok(public_names)
}

fn case_insensitive_cmp(left: &String, right: &String) -> Ordering {
    let cmp = left.to_lowercase().cmp(&right.to_lowercase());
    if let Ordering::Equal = cmp {
        left.cmp(&right)
    } else {
        cmp
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn public_names() {
        let src = "
C = 2
a = 1
def foo():
    ...
__all__ = []
_fooey = 3
bar = 3
";
        assert_eq!(
            get_public_names(src).unwrap(),
            vec![
                String::from("a"),
                String::from("bar"),
                String::from("C"),
                String::from("foo"),
            ]
        );
    }

    #[test]
    fn name_sort() {
        let mut names = vec![
            String::from("foo_car"),
            String::from("A"),
            String::from("foo_bar"),
            String::from("foo"),
            String::from("c"),
            String::from("bAba"),
            String::from("b"),
            String::from("B"),
            String::from("bAbA"),
            String::from("C"),
        ];
        names.sort_by(case_insensitive_cmp);
        assert_eq!(
            names,
            vec![
                String::from("A"),
                String::from("B"),
                String::from("b"),
                String::from("bAbA"),
                String::from("bAba"),
                String::from("C"),
                String::from("c"),
                String::from("foo"),
                String::from("foo_bar"),
                String::from("foo_car"),
            ]
        );
    }

    #[test]
    fn insert_new_allways_block_test() {
        let src = "
A = 1
def foo():
    ...
";
        assert_eq!(
            do_it_allways(src).unwrap().unwrap().as_str(),
            "
A = 1
def foo():
    ...


# allways: start
__all__ = [
    \"A\",
    \"foo\",
]
# allways: end
"
        );
    }

    #[test]
    fn update_allways_block_without_tail() {
        let src = "
A = 1
def foo():
    ...
class Fooey:
    ...


# allways: start
__all__ = [
    \"A\",
    \"foo\",
]
# allways: end
";
        assert_eq!(
            do_it_allways(src).unwrap().unwrap().as_str(),
            "
A = 1
def foo():
    ...
class Fooey:
    ...


# allways: start
__all__ = [
    \"A\",
    \"foo\",
    \"Fooey\",
]
# allways: end
"
        );
    }

    #[test]
    fn update_allways_block_with_tail() {
        let src = "
A = 1
def foo():
    ...
class Fooey:
    ...


# allways: start
__all__ = [
    \"A\",
    \"foo\",
]
# allways: end

import sys, os
";
        assert_eq!(
            do_it_allways(src).unwrap().unwrap().as_str(),
            "
A = 1
def foo():
    ...
class Fooey:
    ...


# allways: start
__all__ = [
    \"A\",
    \"foo\",
    \"Fooey\",
    \"os\",
    \"sys\",
]
# allways: end

import sys, os
"
        );
    }
}
