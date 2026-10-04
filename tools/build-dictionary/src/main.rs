use lexi_core::Dictionary;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Deserialize)]
struct KaikkiEntry {
    word: String,
    pos: Option<String>,
    senses: Option<Vec<KaikkiSense>>,
    translations: Option<Vec<KaikkiTranslation>>,
}

#[derive(Deserialize)]
struct KaikkiSense {
    glosses: Option<Vec<String>>,
    examples: Option<Vec<KaikkiExample>>,
}

#[derive(Deserialize)]
struct KaikkiExample {
    text: Option<String>,
}

#[derive(Deserialize)]
struct KaikkiTranslation {
    lang: Option<String>,
    code: Option<String>,
    word: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Lexi Dictionary Builder Tool");
        println!("Uso: build-dictionary <formato: jsonl|csv> <caminho_arquivo_entrada> [caminho_sqlite_saida]");
        println!("Exemplo: build-dictionary jsonl kaikki.org-dictionary-English.jsonl dictionary.sqlite");
        println!("Exemplo: build-dictionary csv gaming_terms.csv dictionary.sqlite");
        return Ok(());
    }

    let format = &args[1];
    let input_path = &args[2];
    let output_path = if args.len() > 3 { &args[3] } else { "dictionary.sqlite" };

    println!("Criando/abrindo banco SQLite: {}", output_path);
    let dict = Dictionary::open(output_path)?;

    match format.as_str() {
        "jsonl" => import_kaikki_jsonl(&dict, input_path)?,
        "csv" => import_csv(&dict, input_path)?,
        _ => eprintln!("Formato desconhecido: {}. Use 'jsonl' ou 'csv'", format),
    }

    println!("Importação concluída com sucesso para: {}", output_path);
    Ok(())
}

fn import_kaikki_jsonl(dict: &Dictionary, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut imported = 0;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        if let Ok(entry) = serde_json::from_str::<KaikkiEntry>(&line) {
            let mut pt_translation = None;
            if let Some(trans_list) = entry.translations {
                for t in trans_list {
                    if t.code.as_deref() == Some("pt") || t.lang.as_deref() == Some("Portuguese") {
                        if let Some(w) = t.word {
                            pt_translation = Some(w);
                            break;
                        }
                    }
                }
            }

            let mut definition = String::new();
            let mut example = None;
            if let Some(senses) = entry.senses {
                if let Some(first_sense) = senses.first() {
                    if let Some(ref glosses) = first_sense.glosses {
                        definition = glosses.join("; ");
                    }
                    if let Some(ref examples) = first_sense.examples {
                        if let Some(ex) = examples.first() {
                            example = ex.text.clone();
                        }
                    }
                }
            }

            if let Some(trans) = pt_translation {
                let _ = dict.insert_entry(
                    &entry.word,
                    None,
                    entry.pos.as_deref(),
                    &trans,
                    &definition,
                    example.as_deref(),
                );
                imported += 1;
                if imported % 10000 == 0 {
                    println!("Processadas {} palavras com tradução para português...", imported);
                }
            }
        }
    }

    println!("Total importado: {} entradas.", imported);
    Ok(())
}

fn import_csv(dict: &Dictionary, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut count = 0;
    for result in rdr.records() {
        let record = result?;
        if record.len() >= 3 {
            let term = &record[0];
            let pos = if record.len() > 1 { Some(&record[1]) } else { None };
            let trans = &record[2];
            let def = if record.len() > 3 { &record[3] } else { trans };
            let ex = if record.len() > 4 { Some(&record[4]) } else { None };

            dict.insert_entry(term, None, pos, trans, def, ex)?;
            count += 1;
        }
    }
    println!("Total importado do CSV: {} entradas.", count);
    Ok(())
}
