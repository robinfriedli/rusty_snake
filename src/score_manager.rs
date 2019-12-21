use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Cursor, Write};
use std::path::Path;

use quick_xml::events::{Event, BytesStart, BytesEnd};
use quick_xml::{Reader, Writer};

use crate::Difficulty;
use quick_xml::events::attributes::Attribute;
use std::fs;

pub struct ScoreManager<'a> {
    file_path: &'a str
}

impl<'a> ScoreManager<'a> {
    pub fn from_file(file_path: &'a str) -> ScoreManager<'a> {
        let target_path = Path::new(file_path);
        if !target_path.exists() {
            File::create(target_path).expect("could not create scores file");
            fs::copy("scores-template.xml", file_path).expect("failed to create scores file from template");
        }
        return ScoreManager { file_path };
    }

    /// Returns the highest n (defined by the limit param) scores in descending order for the
    /// selected difficulty as a vector of tuples with the score + user name
    pub fn get_high_scores(&mut self, difficulty: &Difficulty, limit: usize) -> Vec<(u64, String)> {
        let mut xml_reader = self.create_reader();
        let mut buf = Vec::new();
        let mut is_reading_relevant_difficulty = false;
        let mut relevant_scores: Vec<(u64, String)> = Vec::new();

        loop {
            match xml_reader.read_event(&mut buf) {
                // b"" returns the string as u8 byte array
                Ok(Event::Start(ref elem)) => {
                    if elem.name() == b"difficulty" {
                        let found_name_attr = Self::get_name_atr(elem);

                        match found_name_attr {
                            Some(res) => {
                                if res.unescape_and_decode_value(&xml_reader).unwrap() == difficulty.to_string() {
                                    is_reading_relevant_difficulty = true;
                                } else {
                                    is_reading_relevant_difficulty = false;
                                }
                            }
                            None => {}
                        }
                    }
                }
                Ok(Event::Empty(ref elem)) => {
                    if elem.name() == b"score" && is_reading_relevant_difficulty {
                        let mut set_score: Option<u64> = None;
                        let mut set_name: Option<String> = None;

                        for attr in elem.attributes() {
                            let attribute: Attribute = attr.unwrap();

                            if attribute.key == b"score" {
                                set_score = Some(attribute.unescape_and_decode_value(&xml_reader).unwrap().parse().unwrap());
                            } else if attribute.key == b"user" {
                                set_name = Some(attribute.unescape_and_decode_value(&xml_reader).unwrap());
                            }
                        }

                        match set_score {
                            Some(score) => {
                                match set_name {
                                    Some(name) => {
                                        relevant_scores.push((score, name));
                                    }
                                    None => {}
                                }
                            }
                            None => {}
                        }
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }

        relevant_scores.sort_by_key(|tuple| tuple.0);
        relevant_scores.reverse();
        relevant_scores.truncate(limit);

        return relevant_scores;
    }

    pub fn write_score(&mut self, score: u64, difficulty: &Difficulty, user_name: &str) {
        let mut xml_reader = self.create_reader();
        let mut buf = Vec::new();
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        let mut difficulty_elem_exists = false;
        loop {
            match xml_reader.read_event(&mut buf) {
                Ok(Event::Start(elem)) => {
                    if elem.name() == b"difficulty" {
                        let name_atr = Self::get_name_atr(&elem);

                        match name_atr {
                            Some(atr) => {
                                let is_current_difficulty = atr.unescape_and_decode_value(&xml_reader).unwrap() == difficulty.to_string();
                                // need to borrow before move
                                writer.write_event(Event::Start(elem)).expect("failed to write elem");
                                if is_current_difficulty {
                                    difficulty_elem_exists = true;
                                    let score_elem = Self::create_score_elem(score, user_name);

                                    writer.write_event(Event::Empty(score_elem)).expect("failed to write elem");
                                }
                            }
                            None => {
                                writer.write_event(Event::Start(elem)).expect("failed to write elem");
                            }
                        }
                    } else {
                        writer.write_event(Event::Start(elem)).expect("failed to write elem");
                    }
                }
                Ok(Event::End(elem)) => {
                    if elem.name() == b"scores" && !difficulty_elem_exists {
                        let mut difficulty_elem = BytesStart::owned(b"difficulty".to_vec(), "difficulty".len());

                        difficulty_elem.push_attribute(("name", difficulty.to_string().as_str()));
                        let score_elem = Self::create_score_elem(score, user_name);

                        writer.write_event(Event::Start(difficulty_elem)).expect("failed to write elem");
                        writer.write_event(Event::Empty(score_elem)).expect("failed to write elem");
                        writer.write_event(Event::End(BytesEnd::borrowed(b"difficulty"))).expect("failed to write elem");
                    }
                    writer.write_event(Event::End(elem)).expect("failed to write elem");
                }
                Ok(Event::Eof) => {
                    break;
                }
                Ok(e) => {
                    writer.write_event(&e).expect("failed to write elem");
                }
                _ => {}
            }
        }

        let mut file = File::create("scores.xml").expect("could not open score file");
        let bytes = writer.into_inner().into_inner();
        file.write_all(bytes.as_slice()).expect("failed writing score to file");
    }

    fn create_reader(&self) -> Reader<BufReader<File>> {
        let score_file = OpenOptions::new().read(true).write(true).create(true).open(self.file_path).expect("Failed to open score file");
        let mut file_reader = BufReader::new(&score_file);
        let mut xml_content: String = String::from("");
        file_reader.read_to_string(&mut xml_content).expect("Unable to read xml file");

        Reader::from_file(Path::new(self.file_path)).expect("failed to initialize xml reader")
    }

    fn get_name_atr<'b>(elem: &'b BytesStart) -> Option<Attribute<'b>> {
        return elem.attributes()
            .map(|attr| attr.unwrap())
            .find(|attr| {
                attr.key == b"name"
            });
    }

    fn create_score_elem(score: u64, user_name: &str) -> BytesStart {
        let mut score_elem = BytesStart::owned(b"score".to_vec(), "score".len());

        score_elem.push_attribute(("score", score.to_string().as_str()));
        score_elem.push_attribute(("user", user_name));

        return score_elem;
    }
}