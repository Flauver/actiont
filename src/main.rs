use std::{collections::{HashMap, HashSet}, fs::{self, read_to_string}};
use rayon::prelude::*;
use itertools::Itertools;

#[derive(serde::Deserialize)]
struct Settings {
    select_keys: Vec<String>
}

fn main() {
    let (mut table, mut reverse) = processing_into_mapping_tables(read_to_string("data/码表.txt").unwrap());
    let sentences: Vec<String> = split_on_punctuation(read_to_string("data/语料.txt").unwrap(), &table);
    let settings: Settings = serde_json::from_str(fs::read_to_string("settings.json").unwrap().as_str()).unwrap();
    let mut entries: HashMap<String, String> = HashMap::with_capacity(100000);
    add_words(&mut table, &mut reverse, &mut entries, &sentences);
    loop {
        let generated_sentence = analog_whole_sentence_engine(&sentences, &table, &reverse, &settings);
        let new_entries = compare(&sentences, generated_sentence, &table);
        if new_entries.difference(&table.keys().cloned().collect::<HashSet<String>>()).collect::<HashSet<&String>>().is_empty() {
            break;
        }
        for entry in new_entries {
            let code: String = entry.chars().map(|c| table[&c.to_string()].clone()).collect::<String>();
            table.insert(entry.clone(), code.clone());
            reverse.insert(code.clone(), entry.clone());
            entries.insert(entry, code);
        }
    }
    fs::write("词条.txt", entries.iter().sorted_by_key(|&(_, code)| code).map(|(word, code)| format!("{}\t{}", word, code)).join("\n")).unwrap();
}
fn split_on_punctuation(text: String, table: &HashMap<String, String>) -> Vec<String> {
    let punctuation: &[char] = &[' ', '\r', '\n', '.', '!', '?', ',', ';', ':', '…', '。', '？', '！', '，', '、', '；', '：', '“', '”', '‘', '’', '「', '」', '『', '』', '—', '《', '》', '〈', '〉', '【', '】', '〔', '〕', '（', '）', '［', '］', '｛', '｝', '〈', '〉', '《', '》', '（', '）', '［', '］', '｛', '｝', '〔', '〕', '〈', '〉'];
    text.split(punctuation)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().filter(|c| table.contains_key(&c.to_string())).collect::<String>())
        .filter(|s| !s.is_empty())
        .collect()
}

fn processing_into_mapping_tables(text: String) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut table: HashMap<String, String> = HashMap::with_capacity(8000);
    let mut reverse: HashMap<String, String> = HashMap::with_capacity(8000);
    for line in text.trim().lines().filter(|s| !s.is_empty()) {
        let (word, code) = line.split_once('\t').unwrap();
        if !table.contains_key(word) {
            table.insert(word.to_string(), code.to_string());
        }
        if !reverse.contains_key(code) {
            reverse.insert(code.to_string(), word.to_string());
        }
    }
    (table, reverse)
}

fn add_words(table: &mut HashMap<String, String>, reverse: &mut HashMap<String, String>, entries: &mut HashMap<String, String>, sentences: &Vec<String>) {
    let mut first = HashSet::with_capacity(8000);
    for word in reverse.values().collect::<Vec<_>>().into_iter() {
        first.insert(word.clone());
    }
    for sentence in sentences {
        let chars = sentence.chars().collect::<Vec<_>>();
        if chars.len() == 1 {
            continue;
        }
        for (i, char) in chars.iter().enumerate() {
            if first.contains(&char.to_string()) {
                continue;
            }
            if i != chars.len() - 1 {
                let word = chars[i..i + 2].iter().collect::<String>();
                let code = table[&char.to_string()].clone() + &table[&chars[i + 1].to_string()].clone();
                table.insert(word.clone(), code.clone());
                reverse.insert(code.clone(), word.clone());
                entries.insert(word, code);
            } else {
                let word = chars[i - 1..i + 1].iter().collect::<String>();
                let code = table[&chars[i - 1].to_string()].clone()+ &table[&char.to_string()].clone();
                table.insert(word.clone(), code.clone());
                reverse.insert(code.clone(), word.clone());
                entries.insert(word, code);
            }
        }
    }
}

fn analog_whole_sentence_engine(sentences: &Vec<String>, table: &HashMap<String, String>, reverse: &HashMap<String, String>, settings: &Settings) -> Vec<String> {
    sentences.par_iter()
        .map(|sentence| {
            let code = sentence.chars().map(|c| table[&c.to_string()].clone()).collect::<String>();
            forward_max_matching_and_mapping(&code, reverse, settings).join("")
        })
        .collect()
}

fn forward_max_matching_and_mapping(text: &String, reverse: &HashMap<String, String>, settings: &Settings) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(10);
    let mut start: usize = 0;
    let max_code_length = reverse.keys().map(|s| s.len()).max().unwrap() as usize;
    while start < text.len() {
        let mut matched = false;
        for end in (start..std::cmp::min(start + max_code_length as usize, text.len()) + 1).rev() {
            if end != text.len() {
                if settings.select_keys.contains(&text[end..end + 1].to_string()) {
                    continue;
                }
            }
            if reverse.contains_key(&text[start..end]) {
                result.push(reverse[&text[start..end]].clone());
                start = end;
                matched = true;
                break;
            }
        }
        if !matched {
            start += 1;
        }
    }
    result
}


fn compare(sentences: &Vec<String>, generated_sentences: Vec<String>, table: &HashMap<String, String>) -> HashSet<String> {
    let mut result: HashSet<String> = HashSet::with_capacity(300000);
    let mut number_of_errors = 0;
    (0..sentences.len()).into_par_iter().map(|i| {
        let mut result: HashSet<String> = HashSet::with_capacity(5);
        let mut number_of_errors = 0;
        let sentence = &sentences[i];
        let generated_sentence = &generated_sentences[i];
        let sentence_chars: Vec<char> = sentence.chars().collect();
        let generated_sentence_chars: Vec<char> = generated_sentence.chars().collect();
        if sentence_chars.len() != generated_sentence_chars.len() {
            let charset: HashSet<char> = sentence_chars.iter().cloned().collect();
            let generated_charset: HashSet<char> = generated_sentence_chars.iter().cloned().collect();
            let mut diff_parts = Vec::with_capacity(10);
            if charset.len() == sentence_chars.len() {
                let same_chars: HashSet<char> = charset.intersection(&generated_charset).cloned().collect();
                diff_parts = split(&sentence, same_chars);
            } else {
                for (i, (&c1, &c2)) in sentence_chars.iter().zip(generated_sentence_chars.iter()).enumerate() {
                    if c1 != c2 {
                        diff_parts.push(sentence_chars[i..].iter().collect::<String>());
                        break;
                    }
                }
            }
            for part in diff_parts {
                let part_chars = part.chars().collect::<Vec<char>>();
                let mut wordlen: usize = 0;
                while wordlen < part_chars.len() {
                    wordlen += 1;
                    if !table.contains_key(&part_chars[0..wordlen].iter().collect::<String>()) {
                        break;
                    }
                }
                if wordlen > 1 {
                    result.insert(part_chars[0..wordlen].iter().collect::<String>());
                    number_of_errors += 1;
                }
            }
        } else {
            let mut diffs = Vec::with_capacity(10);
            for (i, (&c1, &c2)) in sentence_chars.iter().zip(generated_sentence_chars.iter()).enumerate() {
                if c1 != c2 {
                    diffs.push(i);
                }
            }
            if diffs.is_empty() { return (result, number_of_errors); }
            let mut ranges = Vec::with_capacity(5);
            let (mut start, mut end) = (diffs[0], diffs[0]);
            for &index in diffs.iter().skip(1) {
                if index == end + 1 { end = index }
                else { 
                    if end != sentence_chars.len() - 1 {
                        ranges.push((start, end + 1));
                    } else {
                        ranges.push((start, end));
                    }
                    start = index; end = index 
                }
            }
            ranges.push((start, end));

            ranges.iter().map(|&(s,e)| {
                sentence_chars[s..=e].iter()
                    .collect::<String>()
            }).for_each(|s| {
                let part_chars = s.chars().collect::<Vec<char>>();
                let mut wordlen: usize = 0;
                while wordlen < part_chars.len() {
                    wordlen += 1;
                    if !table.contains_key(&part_chars[0..wordlen].iter().collect::<String>()) {
                        break;
                    }
                }
                if wordlen > 1 {
                    result.insert(part_chars[0..wordlen].iter().collect::<String>());
                    number_of_errors += 1;
                }
            });
        }
        (result, number_of_errors)
    }).collect::<Vec<(HashSet<String>, i32)>>().into_iter().for_each(|(thread_result, thread_number_of_errors)| {
        result.extend(thread_result);
        number_of_errors += thread_number_of_errors;
    });
    println!("错误数: {}", number_of_errors);
    result
}

fn split(text: &String, delimiters: HashSet<char>) -> Vec<String> {
    if delimiters.len() == 0 {
        return vec![text.clone()];
    }
    
    let mut result = Vec::with_capacity(10);
    let mut s = String::new();
    for c in text.chars() {
        s.push(c);
        if delimiters.contains(&c) {
            if !s.is_empty() {
                result.push(s.clone());
                s.clear();
            }
        }
    }
    if !s.is_empty() {
        result.push(s);
    }
    result
}
