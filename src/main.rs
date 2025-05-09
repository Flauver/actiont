use std::fs::{self, read_to_string};
use rayon::prelude::*;
use itertools::Itertools;

#[derive(serde::Deserialize)]
struct Settings {
    select_keys: Vec<String>
}

type HashMap<K, V> = std::collections::HashMap<K, V, rustc_hash::FxBuildHasher>;
type HashSet<T> = std::collections::HashSet<T, rustc_hash::FxBuildHasher>;

fn main() {
    let (mut table, mut reverse) = processing_into_mapping_tables(read_to_string("data/码表.txt").unwrap());
    let sentences: Vec<String> = split_on_punctuation(read_to_string("data/语料.txt").unwrap(), &table);
    let settings: Settings = serde_json::from_str(fs::read_to_string("settings.json").unwrap().as_str()).unwrap();
    let mut entries: HashMap<String, String> = HashMap::default();
    let mut endcode: HashSet<String> = HashSet::default();
    loop {
        let generated_sentence = analog_whole_sentence_engine(&sentences, &table, &reverse, &settings);
        let new_entries = compare(&sentences, generated_sentence, &table);
        if new_entries.iter().map(|(x, _)| x).collect::<HashSet<_>>().difference(&table.keys().collect::<HashSet<&String>>()).collect::<HashSet<&&String>>().is_empty() {
            break;
        }
        for (entry, isend) in new_entries {
            let code: String = entry.chars().map(|c| table[&c.to_string()].clone()).collect::<String>();
            table.insert(entry.clone(), code.clone());
            if !endcode.contains(&code) {
                reverse.insert(code.clone(), entry.clone());
                entries.insert(code.clone(), entry);
            }
            if isend {
                endcode.insert(code);
            }
        }
    }
    fs::write("词条.txt", entries.iter().sorted_by_key(|&(code, _)| code).map(|(code, word)| format!("{}\t{}", word, code)).join("\n")).unwrap();
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
    let mut table: HashMap<String, String> = HashMap::default();
    let mut reverse: HashMap<String, String> = HashMap::default();
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

fn analog_whole_sentence_engine(sentences: &Vec<String>, table: &HashMap<String, String>, reverse: &HashMap<String, String>, settings: &Settings) -> Vec<Vec<Vec<char>>> {
    sentences.par_iter()
        .map(|sentence| {
            let code = sentence.chars().map(|c| table[&c.to_string()].clone()).collect::<String>();
            forward_max_matching_and_mapping(&code, reverse, settings)
        })
        .collect()
}

fn forward_max_matching_and_mapping(text: &String, reverse: &HashMap<String, String>, settings: &Settings) -> Vec<Vec<char>> {
    let mut result: Vec<Vec<char>> = Vec::with_capacity(10);
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
                result.push(reverse[&text[start..end]].chars().collect::<Vec<_>>());
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


fn compare(sentences: &Vec<String>, generated_sentences: Vec<Vec<Vec<char>>>, table: &HashMap<String, String>) -> HashSet<(String, bool)> {
    let mut result = HashSet::default();
    (0..sentences.len()).into_par_iter().map(|i| {
        let sentence = sentences[i].chars().collect::<Vec<_>>();
        let generated_sentence = &generated_sentences[i];
        if sentence.len() < 2 {
            return None;
        }
        let mut j = 0;
        let mut lastj = 0;
        let mut k = 0;
        for word in generated_sentence {
            for char in word {
                if *char != sentence[k] {
                    let mut word_len = 2;
                    loop {
                        if j + word_len > sentence.len() {
                            return Some((sentence[lastj..j + word_len - 1].iter().map(|c| c.clone()).collect::<String>(), true));
                        }
                        let word = sentence[j..j + word_len].iter().map(|c| c.clone()).collect::<String>();
                        if !table.contains_key(&word) {
                            if j + word_len == sentence.len() {
                                return Some((word, true));
                            } else {
                                return Some((word, false));
                            }
                        }
                        word_len += 1;
                    }
                }
                k += 1;
            }
            lastj = j;
            j += word.len();
        }
        None
    }).collect::<Vec<Option<(String, bool)>>>().into_iter().for_each(|word| {
        if let Some(word) = word {
            result.insert(word);
        }
    });
    println!("错误数: {}", result.len());
    result
}
