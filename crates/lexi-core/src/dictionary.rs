use crate::models::{LookupResult, LookupSource};
use flate2::read::ZlibDecoder;
use rusqlite::{params, Connection, OpenFlags};
use std::io::Read;
use std::path::Path;

const EMBEDDED_DICT_BYTES: &[u8] = include_bytes!("../data/dictionary.bin");

pub struct Dictionary {
    conn: Connection,
}

impl Dictionary {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let path_ref = path.as_ref();
        let conn = match Connection::open(path_ref) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Could not open database at {:?} in read-write mode: {}. Attempting read-only/immutable...", path_ref, e);
                if path_ref.exists() {
                    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
                    let clean_path = path_ref.to_string_lossy().replace('\\', "/");
                    let uri = if clean_path.starts_with('/') {
                        format!("file:{}?mode=ro&immutable=1", clean_path)
                    } else {
                        format!("file:///{}?mode=ro&immutable=1", clean_path)
                    };
                    Connection::open_with_flags(&uri, flags)?
                } else {
                    return Err(e);
                }
            }
        };

        // Performance & concurrency pragmas: WAL mode, 64MB cache, 256MB mmap
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA mmap_size = 268435456;
             PRAGMA temp_store = MEMORY;"
        );

        let dict = Self { conn };
        let _ = dict.init_schema();
        let _ = dict.ensure_starter_seeds();
        Ok(dict)
    }

    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let _ = conn.execute_batch(
            "PRAGMA synchronous = OFF;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;"
        );
        let dict = Self { conn };
        dict.init_schema()?;
        dict.ensure_starter_seeds()?;
        Ok(dict)
    }

    pub fn entry_count(&self) -> i64 {
        self.conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0)).unwrap_or(0)
    }

    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                term TEXT PRIMARY KEY,
                lemma TEXT,
                pos TEXT,
                translation TEXT NOT NULL,
                definition TEXT NOT NULL,
                example TEXT
            );",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entries_lemma ON entries(lemma COLLATE NOCASE);",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entries_translation ON entries(translation COLLATE NOCASE);",
            [],
        )?;
        Ok(())
    }


    pub fn insert_entry(
        &self,
        term: &str,
        lemma: Option<&str>,
        pos: Option<&str>,
        translation: &str,
        definition: &str,
        example: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO entries (term, lemma, pos, translation, definition, example)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![term.to_lowercase(), lemma.map(|s| s.to_lowercase()), pos, translation, definition, example],
        )?;
        Ok(())
    }

    pub fn lookup(&self, query: &str) -> Option<LookupResult> {
        let clean = query.trim().to_lowercase();
        if clean.is_empty() {
            return None;
        }

        // 1. Direct query
        if let Some(res) = self.query_single(&clean) {
            return Some(res);
        }

        // 2. Direct query with hyphen/space variations
        if clean.contains('-') {
            let spaced = clean.replace('-', " ");
            if let Some(res) = self.query_single(&spaced) {
                return Some(res);
            }
            let continuous = clean.replace('-', "");
            if let Some(res) = self.query_single(&continuous) {
                return Some(res);
            }
        } else if clean.contains(' ') {
            let hyphenated = clean.replace(' ', "-");
            if let Some(res) = self.query_single(&hyphenated) {
                return Some(res);
            }
            let continuous = clean.replace(' ', "");
            if let Some(res) = self.query_single(&continuous) {
                return Some(res);
            }
        }

        // 3. Simple lemmatization heuristics (English regular plurals, past tense, progressive)
        let candidates = self.generate_lemma_candidates(&clean);
        for candidate in candidates {
            if let Some(res) = self.query_single(&candidate) {
                let mut adjusted = res;
                adjusted.query = query.to_string();
                return Some(adjusted);
            }
        }

        // 4. Reverse query: Portuguese translation match (only if at least 3 characters)
        if clean.len() >= 3 {
            if let Some(res) = self.query_by_translation(&clean) {
                return Some(res);
            }
        }

        None
    }

    fn query_single(&self, word: &str) -> Option<LookupResult> {
        let mut stmt = match self.conn.prepare(
            "SELECT term, pos, translation, definition, example FROM entries WHERE term = ?1 LIMIT 1"
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("SQLite prepare error in query_single for '{}': {}", word, e);
                return None;
            }
        };

        let mut rows = match stmt.query(params![word]) {
            Ok(r) => r,
            Err(e) => {
                log::error!("SQLite query error in query_single for '{}': {}", word, e);
                return None;
            }
        };

        match rows.next() {
            Ok(Some(row)) => {
                let term: String = match row.get(0) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("SQLite get term error: {}", e);
                        return None;
                    }
                };
                let pos: Option<String> = row.get::<_, Option<String>>(1).ok().flatten().filter(|s| !s.trim().is_empty());
                let translation: String = match row.get(2) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("SQLite get translation error: {}", e);
                        return None;
                    }
                };
                let definition: String = match row.get(3) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("SQLite get definition error: {}", e);
                        return None;
                    }
                };
                let example: Option<String> = row.get::<_, Option<String>>(4).ok().flatten().filter(|s| !s.trim().is_empty());

                let clean_def = if definition.trim().is_empty() || definition.trim().eq_ignore_ascii_case(translation.trim()) {
                    format!("Termo em inglês que se traduz como '{}'.", translation)
                } else {
                    definition
                };

                Some(LookupResult {
                    query: term.clone(),
                    normalized: term,
                    translation,
                    part_of_speech: pos,
                    definition: clean_def,
                    example,
                    source: LookupSource::OfflineDictionary,
                })
            }
            Ok(None) => None,
            Err(e) => {
                log::error!("SQLite next row error in query_single for '{}': {}", word, e);
                None
            }
        }
    }

    fn query_by_translation(&self, word: &str) -> Option<LookupResult> {
        if word.len() < 3 {
            return None;
        }

        let mut capitalized = String::with_capacity(word.len());
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            capitalized.extend(first.to_uppercase());
            capitalized.push_str(chars.as_str());
        }

        // Fast indexed exact match only (<2ms).
        // Uses idx_entries_translation without any table scanning.
        if let Ok(mut stmt) = self.conn.prepare(
            "SELECT term, pos, translation, definition, example FROM entries 
             WHERE translation = ?1 OR translation = ?2 
             ORDER BY length(term) ASC 
             LIMIT 1"
        ) {
            if let Ok(mut rows) = stmt.query(params![word, capitalized]) {
                if let Ok(Some(row)) = rows.next() {
                    if let (Ok(term), Ok(translation), Ok(definition)) = (
                        row.get::<_, String>(0),
                        row.get::<_, String>(2),
                        row.get::<_, String>(3),
                    ) {
                        let pos = row.get::<_, Option<String>>(1).ok().flatten().filter(|s| !s.trim().is_empty());
                        let example = row.get::<_, Option<String>>(4).ok().flatten().filter(|s| !s.trim().is_empty());
                        let clean_def = if definition.trim().is_empty() || definition.trim().eq_ignore_ascii_case(translation.trim()) {
                            format!("Termo em inglês que se traduz como '{}'.", translation)
                        } else {
                            definition
                        };
                        return Some(LookupResult {
                            query: word.to_string(),
                            normalized: term,
                            translation,
                            part_of_speech: pos,
                            definition: clean_def,
                            example,
                            source: LookupSource::OfflineDictionary,
                        });
                    }
                }
            }
        }

        None
    }


    fn generate_lemma_candidates(&self, word: &str) -> Vec<String> {
        let mut list = Vec::new();

        // 1. Irregular mappings (verbs, plurals, comparatives)
        match word {
            "was" | "were" | "been" | "am" | "are" | "is" => list.push("be".to_string()),
            "had" | "has" | "having" => list.push("have".to_string()),
            "did" | "does" | "done" | "doing" => list.push("do".to_string()),
            "said" | "says" | "saying" => list.push("say".to_string()),
            "went" | "goes" | "gone" | "going" => list.push("go".to_string()),
            "got" | "gotten" | "gets" | "getting" => list.push("get".to_string()),
            "made" | "makes" | "making" => list.push("make".to_string()),
            "knew" | "known" | "knows" | "knowing" => list.push("know".to_string()),
            "took" | "taken" | "takes" | "taking" => list.push("take".to_string()),
            "saw" | "seen" | "sees" | "seeing" => list.push("see".to_string()),
            "came" | "comes" | "coming" => list.push("come".to_string()),
            "thought" | "thinks" | "thinking" => list.push("think".to_string()),
            "looked" | "looks" | "looking" => list.push("look".to_string()),
            "wanted" | "wants" | "wanting" => list.push("want".to_string()),
            "gave" | "given" | "gives" | "giving" => list.push("give".to_string()),
            "used" | "uses" | "using" => list.push("use".to_string()),
            "found" | "finds" | "finding" => list.push("find".to_string()),
            "told" | "tells" | "telling" => list.push("tell".to_string()),
            "asked" | "asks" | "asking" => list.push("ask".to_string()),
            "worked" | "works" | "working" => list.push("work".to_string()),
            "seemed" | "seems" | "seeming" => list.push("seem".to_string()),
            "felt" | "feels" | "feeling" => list.push("feel".to_string()),
            "tried" | "tries" | "trying" => list.push("try".to_string()),
            "left" | "leaves" | "leaving" => list.push("leave".to_string()),
            "called" | "calls" | "calling" => list.push("call".to_string()),
            "could" => list.push("can".to_string()),
            "would" => list.push("will".to_string()),
            "should" => list.push("shall".to_string()),
            "bought" | "buys" | "buying" => list.push("buy".to_string()),
            "brought" | "brings" | "bringing" => list.push("bring".to_string()),
            "began" | "begun" | "begins" | "beginning" => list.push("begin".to_string()),
            "kept" | "keeps" | "keeping" => list.push("keep".to_string()),
            "held" | "holds" | "holding" => list.push("hold".to_string()),
            "wrote" | "written" | "writes" | "writing" => list.push("write".to_string()),
            "stood" | "stands" | "standing" => list.push("stand".to_string()),
            "heard" | "hears" | "hearing" => list.push("hear".to_string()),
            "meant" | "means" | "meaning" => list.push("mean".to_string()),
            "met" | "meets" | "meeting" => list.push("meet".to_string()),
            "ran" | "runs" | "running" => list.push("run".to_string()),
            "paid" | "pays" | "paying" => list.push("pay".to_string()),
            "sat" | "sits" | "sitting" => list.push("sit".to_string()),
            "spoke" | "spoken" | "speaks" | "speaking" => list.push("speak".to_string()),
            "lost" | "loses" | "losing" => list.push("lose".to_string()),
            "fell" | "fallen" | "falls" | "falling" => list.push("fall".to_string()),
            "sent" | "sends" | "sending" => list.push("send".to_string()),
            "built" | "builds" | "building" => list.push("build".to_string()),
            "spent" | "spends" | "spending" => list.push("spend".to_string()),
            "drew" | "drawn" | "draws" | "drawing" => list.push("draw".to_string()),
            "broke" | "broken" | "breaks" | "breaking" => list.push("break".to_string()),
            "won" | "wins" | "winning" => list.push("win".to_string()),
            "chose" | "chosen" | "chooses" | "choosing" => list.push("choose".to_string()),
            "drove" | "driven" | "drives" | "driving" => list.push("drive".to_string()),
            "wore" | "worn" | "wears" | "wearing" => list.push("wear".to_string()),
            "flew" | "flown" | "flies" | "flying" => list.push("fly".to_string()),
            "swam" | "swum" | "swims" | "swimming" => list.push("swim".to_string()),
            "ate" | "eaten" | "eats" | "eating" => list.push("eat".to_string()),
            "drank" | "drunk" | "drinks" | "drinking" => list.push("drink".to_string()),
            "slept" | "sleeps" | "sleeping" => list.push("sleep".to_string()),
            // Irregular plurals
            "men" => list.push("man".to_string()),
            "women" => list.push("woman".to_string()),
            "children" => list.push("child".to_string()),
            "feet" => list.push("foot".to_string()),
            "teeth" => list.push("tooth".to_string()),
            "mice" => list.push("mouse".to_string()),
            "people" => list.push("person".to_string()),
            // Irregular comparatives / superlatives
            "better" | "best" => list.push("good".to_string()),
            "worse" | "worst" => list.push("bad".to_string()),
            "more" | "most" => list.push("much".to_string()),
            "less" | "least" => list.push("little".to_string()),
            _ => {}
        }

        // 2. Morphological suffix rules
        if let Some(stem) = word.strip_suffix("ies") {
            list.push(format!("{}y", stem)); // bounties -> bounty
        }
        if let Some(stem) = word.strip_suffix("ied") {
            list.push(format!("{}y", stem)); // replied -> reply, carried -> carry
        }
        if let Some(stem) = word.strip_suffix("ves") {
            list.push(format!("{}f", stem));  // wolves -> wolf, leaves -> leaf
            list.push(format!("{}fe", stem)); // knives -> knife, lives -> life
        }
        if let Some(stem) = word.strip_suffix("ing") {
            list.push(format!("{}e", stem)); // striking -> strike
            list.push(stem.to_string());      // flanking -> flank
            // Handle doubled consonants: running -> run, swimming -> swim, dropping -> drop
            let bytes = stem.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                list.push(stem[..stem.len() - 1].to_string());
            }
        }
        if let Some(stem) = word.strip_suffix("ed") {
            list.push(format!("{}e", stem)); // perishing -> perish / baked -> bake
            list.push(stem.to_string());      // flanked -> flank
            let bytes = stem.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                list.push(stem[..stem.len() - 1].to_string());
            }
        }
        if let Some(stem) = word.strip_suffix("ly") {
            list.push(stem.to_string());      // quickly -> quick
            if let Some(i_stem) = stem.strip_suffix('i') {
                list.push(format!("{}y", i_stem)); // easily -> easy, happily -> happy
            }
        }
        if let Some(stem) = word.strip_suffix("ness") {
            list.push(stem.to_string());      // darkness -> dark
            if let Some(i_stem) = stem.strip_suffix('i') {
                list.push(format!("{}y", i_stem)); // happiness -> happy
            }
        }
        if let Some(stem) = word.strip_suffix("er") {
            list.push(stem.to_string());      // faster -> fast
            if let Some(base) = stem.strip_suffix('e') {
                list.push(base.to_string());
            }
        }
        if let Some(stem) = word.strip_suffix("est") {
            list.push(stem.to_string());      // fastest -> fast
            if let Some(base) = stem.strip_suffix('e') {
                list.push(base.to_string());
            }
        }
        if let Some(stem) = word.strip_suffix('s') {
            if let Some(es) = word.strip_suffix("es") {
                list.push(es.to_string());    // matches -> match, boxes -> box
            }
            list.push(stem.to_string());      // flanks -> flank, books -> book
        }

        list
    }

    fn ensure_starter_seeds(&self) -> Result<(), rusqlite::Error> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0)).unwrap_or(0);
        if count < 50000 {
            self.seed_from_embedded()?;
        }

        // Always ensure enriched gaming seeds take precedence, even for pre-existing databases
        self.seed_gaming_seeds()
    }

    pub fn seed_gaming_seeds(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("BEGIN TRANSACTION;", [])?;
        {
            let mut stmt = self.conn.prepare(
                "INSERT OR REPLACE INTO entries (term, lemma, pos, translation, definition, example)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )?;

            for &(term, pos, trans, defn, ex) in GAMING_STARTER_SEEDS {
                let _ = stmt.execute(params![
                    term.to_lowercase(),
                    term.to_lowercase(),
                    Some(pos),
                    trans,
                    defn,
                    Some(ex),
                ]);
            }
        }
        self.conn.execute("COMMIT;", [])?;
        log::info!("Gaming starter seeds applied successfully ({} terms)", GAMING_STARTER_SEEDS.len());
        Ok(())
    }

    fn seed_from_embedded(&self) -> Result<(), rusqlite::Error> {
        let mut decoder = ZlibDecoder::new(EMBEDDED_DICT_BYTES);
        let mut tsv = String::new();
        if let Err(e) = decoder.read_to_string(&mut tsv) {
            log::error!("Failed to decompress embedded dictionary: {}", e);
            return Ok(());
        }

        self.conn.execute("BEGIN TRANSACTION;", [])?;

        {
            let mut stmt = self.conn.prepare(
                "INSERT OR REPLACE INTO entries (term, lemma, pos, translation, definition, example)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )?;

            for line in tsv.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut parts = trimmed.split('\t');
                let term = parts.next().unwrap_or("").trim();
                if term.is_empty() {
                    continue;
                }
                let pos = parts.next().unwrap_or("").trim();
                let translation = parts.next().unwrap_or("").trim();
                let definition = parts.next().unwrap_or("").trim();
                let example = parts.next().unwrap_or("").trim();

                let pos_opt = if pos.is_empty() { None } else { Some(pos) };
                let ex_opt = if example.is_empty() { None } else { Some(example) };

                let _ = stmt.execute(params![
                    term.to_lowercase(),
                    term.to_lowercase(),
                    pos_opt,
                    translation,
                    definition,
                    ex_opt,
                ]);
            }
        }

        self.conn.execute("COMMIT;", [])?;
        log::info!("Embedded dictionary seeded successfully (15,000+ entries)");
        Ok(())
    }
}

pub const GAMING_STARTER_SEEDS: &[(&str, &str, &str, &str, &str)] = &[
    ("ace", "gíria gamer", "eliminação total / matar todos os adversários", "Feito no qual um único jogador (ou a equipe) elimina todos os integrantes da equipe inimiga em uma rodada ou confronto de equipe.", "He got a clean ace with the pistol in round 2!"),
    ("afk", "gíria gamer", "ausente do teclado (Away From Keyboard)", "Jogador que abandonou o teclado ou permanece inativo sem controlar seu personagem durante o andamento da partida.", "Our support went AFK right before the crucial teamfight."),
    ("aggro", "termo gamer", "atrair a fúria / foco do inimigo", "Atenção e prioridade de ataque que um monstro, chefe ou torre dá a um jogador; 'puxar o aggro' significa forçar o inimigo a focar em você em vez dos seus aliados.", "Wait for the tank to get aggro before dealing damage!"),
    ("aoe", "termo gamer", "área de efeito (Area of Effect)", "Habilidades, magias ou explosões que causam dano ou efeitos em múltiplos alvos dentro de um raio de alcance simultaneamente.", "Use your AoE ultimate when the enemies bunch up together."),
    ("backdoor", "gíria gamer", "invasão surpresa à base / ataque pelas costas", "Estratégia de invadir furtivamente e destruir a estrutura principal da base adversária desguarnecida enquanto o time inimigo está ocupado combatendo noutro lugar.", "While they contested the objective, our rogue pulled off a backdoor and won the match!"),
    ("bait", "gíria gamer", "atrair para armadilha / ser a isca", "Tática de simular fraqueza, errar propositalmente ou se expor como isca para induzir o adversário a cometer um erro e cair em uma emboscada.", "Pretend you are low health to bait them into our bush!"),
    ("buff", "termo gamer", "bônus temporário / fortalecimento de atributos", "Efeito positivo que melhora temporariamente o dano, defesa, velocidade ou cura de um personagem, ou alteração de equilíbrio feita pelo desenvolvedor aumentando o poder de um item ou campeão.", "Grab the damage buff before we initiate the final boss fight."),
    ("build", "termo gamer", "combinação de itens / montagem de atributos", "Conjunto estruturado de armas, armaduras, runas, talentos e atributos escolhidos para otimizar um personagem para um estilo de jogo.", "I'm testing an attack speed and critical strike build this match."),
    ("burst", "termo gamer", "dano massivo instantâneo", "Capacidade de desferir uma quantidade gigantesca de dano em questão de frações de segundo, eliminando o alvo antes que ele possa reagir.", "That assassin has enough burst damage to delete squishy targets."),
    ("camp", "gíria gamer", "acampar / esperar imóvel em um ponto fixo", "Ficar parado esperando por muito tempo em uma esquina ou esconderijo vantajoso para surpreender e eliminar quem passar pelo local.", "Stop camping in that corner with a shotgun!"),
    ("carry", "termo gamer", "carregador / protagonista do time", "Personagem ou jogador encarregado de causar a maior quantidade de dano e levar a equipe à vitória nas fases finais do jogo.", "Protect our carry at all costs, he has full build now."),
    ("cc", "termo gamer", "controle de grupo (Crowd Control)", "Habilidades que limitam ou impedem a movimentação e as ações do oponente, incluindo atordoamentos (stun), lentidões (slow), silêncios e enraizamentos.", "Chain your CC on their carry so he can't flash away!"),
    ("choke point", "termo gamer", "ponto de estrangulamento / gargalo do mapa", "Passagem estreita, corredor ou ponte no mapa onde a movimentação de tropas e jogadores fica afunilada, facilitando ataques em área e armadilhas.", "Hold the choke point and don't let them push into our base!"),
    ("clutch", "gíria gamer", "vitória sob pressão / virada espetacular", "Momento decisivo em que um jogador vence uma rodada ou situação crítica quando está em clara desvantagem numérica (ex.: vencer sozinho contra 3 ou mais adversários).", "What an insane clutch to win the tournament round in a 1v3!"),
    ("cooldown", "termo gamer", "tempo de recarga", "Período obrigatório de espera após o uso de uma habilidade, item ou feitiço antes que ele possa ser ativado novamente.", "My shield is on cooldown for another 5 seconds, back off!"),
    ("creep", "termo gamer", "monstro neutro / criatura do mapa", "Criaturas ou monstros neutros que habitam o mapa e podem ser eliminados pelos jogadores para obtenção de ouro, bônus e experiência.", "Farm the jungle creeps to recover your experience deficit."),
    ("crowd control", "termo gamer", "controle de grupo", "Efeitos mágicos ou físicos capazes de controlar, imobilizar ou desabilitar ações e movimentações dos inimigos durante os combates.", "Our team composition has huge crowd control to lock down enemies."),
    ("debuff", "termo gamer", "penalidade temporária / efeito prejudicial", "Efeito negativo temporário aplicado a um alvo que reduz seus atributos como armadura, dano, velocidade ou poder de cura.", "The boss casts an armor reduction debuff on anyone in front of him."),
    ("dot", "termo gamer", "dano contínuo (Damage over Time)", "Efeito prejudicial (como veneno, sangramento ou chamas) que retira pontos de vida do adversário gradualmente a cada fração de segundo.", "Apply your poison DoTs and let the boss slowly bleed out."),
    ("dps", "termo gamer", "dano por segundo / causador de dano", "Métrica de dano sustentado causado a cada segundo, ou denominação das classes especializadas em infligir altas quantidades de dano contínuo.", "We need more DPS to bring down this raid boss before time runs out."),
    ("drop", "termo gamer", "largar item / recompensa caída", "Ação de soltar uma arma ou equipamento no chão para um colega de time, ou o saque/recompensa deixado por um inimigo derrotado ou caixa de suprimentos.", "Can you drop me a rifle? I don't have enough money."),
    ("eco", "termo gamer", "rodada econômica / economizar créditos", "Rodada em jogos de tiro tático onde o time decide não comprar armas ou equipamentos caros, guardando dinheiro para ter poder de fogo completo nas rodadas seguintes.", "Save your money this round, we are on full eco."),
    ("farming", "gíria gamer", "farmar / coletar ouro e recursos", "Atividade contínua de abater tropas, monstros ou colher recursos de forma eficiente para acumular dinheiro e experiência rapidamente.", "Focus on farming your core items before committing to team fights."),
    ("feed", "gíria gamer", "alimentar o inimigo / morrer repetidamente", "Morrer consecutivas vezes para os adversários, concedendo-lhes ouro e experiência que os deixam muito à frente na partida.", "Stop fighting him 1v1, you are just feeding his assassin!"),
    ("feeder", "gíria gamer", "jogador que morre em excesso", "Jogador que é repetidamente eliminado pelo time adversário, tornando-os excessivamente fortes e desestabilizando o equilíbrio da partida.", "Our top laner is feeding the enemy bruiser."),
    ("flank", "termo gamer", "flanquear / ataque lateral", "Estratégia tática de contornar a linha de frente adversária para atacar pelas laterais ou retaguarda, surpreendendo inimigos desatentos ou alvos frágeis.", "Flank them from behind while we hold the choke point!"),
    ("focus", "termo gamer", "focar alvo prioritário", "Comando tático para concentrar os ataques e habilidades de toda a equipe em um único inimigo de maior perigo (como o curador ou atirador).", "Focus the healer first, don't waste damage on the tank!"),
    ("freeze", "termo gamer", "congelar a rota / segurar tropas", "Técnica tática de controlar as ondas de tropas para que elas se matem perto da sua torre de forma estática, protegendo você e expondo o adversário.", "Freeze the lane just outside our turret range to deny him gold."),
    ("gank", "gíria gamer", "emboscada / ataque surpresa em grupo", "Ataque surpresa coordenado com vantagem numérica contra um inimigo em sua rota ou posição isolada, visando garantir uma eliminação rápida.", "Mid is overextended without vision, let's gank him right now!"),
    ("gg", "gíria gamer", "bom jogo (Good Game)", "Cumprimento tradicional e respeitoso trocado entre competidores ao final de uma partida para agradecer pelo confronto.", "GG WP everyone, that was a really exciting game!"),
    ("hitbox", "termo gamer", "caixa de colisão", "Área volumétrica invisível ao redor de personagens ou objetos que o jogo utiliza para calcular se tiros, golpes ou habilidades acertaram o alvo.", "That character has a very small hitbox, making it hard to land headshots."),
    ("inting", "gíria gamer", "morrendo intencionalmente / feeding voluntário", "Contração de 'intentional feeding'; conduta tóxica de se entregar intencionalmente aos inimigos para que eles ganhem abates fáceis, prejudicando o próprio time.", "Our teammate got mad and started inting down the mid lane."),
    ("jungle", "termo gamer", "selva / jungle / rota da selva", "Área do mapa situada entre as rotas principais (lanes), repleta de monstros neutros e objetivos que concedem ouro, experiência e bônus à equipe.", "Our jungle is invading their blue buff."),
    ("jungler", "termo gamer", "caçador / jogador da selva", "Jogador especializado em derrotar monstros na selva, controlar objetivos neutros (como dragão/barão) e realizar emboscadas (ganks) nas rotas.", "The enemy jungler was spotted on top side river."),
    ("kite", "gíria gamer", "bater e correr / atacar recuando", "Técnica mecânica de desferir ataques no oponente enquanto recua continuamente para fora do alcance dele, causando dano sem receber retaliação.", "Kite the enemy bruiser so he never gets close enough to hit you!"),
    ("lag", "termo gamer", "atraso de resposta / lentidão de rede", "Atraso perceptível entre os comandos executados pelo jogador e a resposta visual no jogo, provocado por conexões instáveis, perda de pacotes ou quedas de taxa de quadros (FPS).", "I lagged right as I pulled the trigger and missed the target."),
    ("lane", "termo gamer", "rota / via do mapa", "Caminho principal traçado no mapa por onde as tropas automatizadas marcham rumo à base inimiga (ex.: top lane, mid lane, bot lane).", "Push your lane hard before recalling to base."),
    ("leash", "gíria gamer", "ajudar no primeiro monstro da selva", "Auxílio dado pelos jogadores de rota ao caçador (jungler) para bater no monstro inicial da selva, facilitando seu começo de partida sem roubar o abate.", "Give our jungler a quick leash on the red buff before minions spawn."),
    ("loadout", "termo gamer", "seleção de equipamento / kit de armamento", "Configuração personalizada de armas primárias, secundárias, granadas e regalias (perks) preparada pelo jogador antes do combate.", "Customize your loadout with a sniper and smoke grenades."),
    ("loot", "termo gamer", "saque / espólio / itens caídos", "Equipamentos, armas, moedas e itens valiosos recolhidos de baús espalhados pelo cenário ou deixados no chão por inimigos derrotados.", "Don't spend too long looting while enemies are nearby."),
    ("meta", "termo gamer", "estratégia predominante (Most Effective Tactic Available)", "Conjunto das estratégias, armas, personagens e composições comprovadamente mais eficientes e populares no estado atual do jogo.", "Double sniper is the dominant meta in competitive play right now."),
    ("minion", "termo gamer", "minion / lacaio da rota", "Pequenas tropas automatizadas que marcham pelas rotas em direção às estruturas adversárias, fornecendo ouro e experiência ao serem abatidas.", "Clear the incoming minion wave to prevent tower damage."),
    ("nerf", "termo gamer", "enfraquecimento / redução de poder", "Redução deliberada na eficácia, dano ou resistência de um personagem, arma ou habilidade feita pelos desenvolvedores em um patch para equilibrar o jogo.", "That sniper rifle received a heavy damage nerf in the latest update."),
    ("op", "gíria gamer", "muito forte / desequilibrado (Overpowered)", "Personagem, arma ou habilidade tão poderosa que supera injustamente todas as outras alternativas disponíveis.", "That new champion's ultimate is completely OP and needs a nerf."),
    ("overextended", "termo gamer", "excessivamente avançado / mal posicionado", "Estar posicionado perigosamente longe da segurança das suas estruturas ou aliados, ficando vulnerável a emboscadas imediatas.", "Fall back! You are overextended without vision in the river."),
    ("peel", "gíria gamer", "proteger aliado / afastar inimigos", "Ação de proteger um aliado mais frágil (como o atirador/carry), usando atordoamentos, lentidões ou repelindo os inimigos que tentam eliminá-lo.", "Peel for our carry, they have assassins flanking!"),
    ("ping", "termo gamer", "latência de rede (ms) / marcação de alerta no mapa", "Tempo de resposta da conexão com o servidor medido em milissegundos, ou sinal sonoro/visual enviado no minimapa para alertar a equipe sobre perigo, ajuda ou objetivos.", "My ping spiked to 250ms! Ping danger on the river bush."),
    ("proc", "gíria gamer", "ativação de efeito especial / disparar probabilidade", "Ativação com base em chance percentual de um efeito especial de arma, armadura ou magia (ex.: chance de 10% de disparar relâmpago ao golpear).", "My weapon proc activated on that critical strike!"),
    ("push", "termo gamer", "avançar na rota / empurrar linha", "Ação de abater tropas inimigas com rapidez para fazer a sua linha avançar, pressionando torres e estruturas do adversário.", "Push the mid lane right now while three enemies are dead!"),
    ("respawn", "termo gamer", "renascer / reaparecer no mapa", "Retorno de um jogador, criatura ou recurso ao mapa do jogo após ter sido eliminado ou esgotado, geralmente condicionado a um temporizador.", "He will respawn at base in 15 seconds."),
    ("roam", "gíria gamer", "rotacionar / percorrer o mapa para ajudar", "Ação de se deslocar da sua rota principal para prestar suporte, gankar ou disputar objetivos neutros em outras regiões do mapa.", "Mid is roaming towards bottom lane, ping missing immediately!"),
    ("skillshot", "termo gamer", "tiro de habilidade / disparo com mira manual", "Projétil ou magia disparada manualmente que exige precisão de mira e antecipação dos movimentos do alvo, em vez de teleguiar automaticamente.", "He landed an amazing skillshot from across the map."),
    ("smurf", "gíria gamer", "jogador veterano em conta de novato", "Jogador experiente de nível ou ranking elevado que joga em uma conta nova ou secundária de elo baixo para enfrentar oponentes novatos sem dificuldade.", "That level 3 player with flawless aim is definitely a smurf."),
    ("spawn kill", "gíria gamer", "matar no ponto de nascimento", "Ação de abater um adversário no instante exato em que ele renasce no ponto de surgimento, sem dar oportunidade de defesa ou locomoção.", "The enemy team is camping our base for spawn kills."),
    ("split push", "termo gamer", "avanço dividido / pressão lateral separada", "Estratégia onde um jogador avança sozinho em uma rota distante enquanto os outros quatro aliados agrupam noutro local, forçando o inimigo a escolher onde se defender.", "Don't fight 4v5, stall them while our top laner does a split push."),
    ("stacking", "termo gamer", "acumular cargas / acúmulo de bônus", "Ato de acumular repetidamente cargas de feitiços, atributos, monstros ou efeitos passivos para potencializar seu resultado final.", "He is stacking passive damage buffs that make his hits devastating."),
    ("stealth", "termo gamer", "furtividade / invisibilidade", "Mecânica ou habilidade que torna o personagem invisível ou camuflado aos olhos dos oponentes até que ele realize um ataque.", "He activated stealth and snuck behind enemy lines."),
    ("support", "termo gamer", "suporte / classe de auxílio", "Papel ou personagem voltado a apoiar aliados com cura, escudos, visão de mapa e controle de grupo, sem priorizar acúmulo de abates para si.", "The support placed wards all around the enemy jungle."),
    ("tank", "termo gamer", "tanque / blindado da linha de frente", "Personagem resistente com foco em vitalidade, armadura e controle de grupo, responsável por iniciar combates e absorver dano pelos aliados.", "Let the tank walk in first to absorb all the enemy skillshots."),
    ("throw", "gíria gamer", "entregar a vitória / jogar fora uma vantagem grande", "Cometer falhas grosseiras ou decisões precipitadas que resultam na derrota de uma partida na qual a equipe possuía uma vantagem praticamente irreversível.", "We had a 15-kill lead, how did we throw that game at the end?"),
    ("tilt", "gíria gamer", "desestabilizado / frustrado emocionalmente", "Estado mental de estresse e frustração que leva o jogador a perder o foco e a paciência, cometendo erros sucessivos e jogando bem abaixo da sua capacidade normal.", "Take a five-minute break after that loss, you're visibly tilted."),
    ("tilted", "gíria gamer", "irritado / no tilt", "Estar sob efeito de tilt, abalado psicologicamente por derrotas ou jogadas desfavoráveis, jogando de maneira agressiva e desordenada.", "He is tilted after being eliminated three times in a row."),
    ("tower dive", "termo gamer", "mergulho sob a torre / abater debaixo da torre", "Manobra arriscada de entrar voluntariamente no alcance dos tiros da torre inimiga para abater um adversário com pouca vida que busca abrigo nela.", "Their carry has only 10% health, let's execute a tower dive!"),
    ("ult", "gíria gamer", "habilidade suprema (ultimate)", "A habilidade mais forte e com maior tempo de recarga de um personagem, capaz de definir o resultado de confrontos em grupo.", "My ult is ready, initiate the fight whenever you are ready!"),
    ("ultimate", "termo gamer", "habilidade definitiva / suprema", "Poder supremo de um herói ou campeão que possui alto impacto e geralmente requer acumular carga ou longo tempo de recarga.", "Combine your ultimate with mine for a devastating combo."),
    ("shyster", "gíria", "trapaceiro / vigarista / indivíduo desonesto", "Pessoa inescrupulosa ou desonesta que age com trapaça ou má-fé em negócios ou jogos.", "Don't trust that shyster with your rare items!"),
    ("sheister", "gíria", "trapaceiro / vigarista / indivíduo desonesto", "Variação fonética de 'shyster'; trapaceiro, vigarista ou jogador que tenta enganar os outros.", "Watch out for that sheister in trade chat!"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_dictionary_in_memory() {
        let dict = Dictionary::in_memory().unwrap();

        let count: i64 = dict.conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0)).unwrap();
        assert!(count > 250000, "Expected at least 250,000 entries, found {}", count);

        // General words reported by user
        let res_env = dict.lookup("environment").expect("'environment' should exist");
        assert_eq!(res_env.normalized, "environment");
        assert!(res_env.translation.to_lowercase().contains("ambiente"));

        let res_test = dict.lookup("test").expect("'test' should exist");
        assert_eq!(res_test.normalized, "test");
        assert!(res_test.translation.to_lowercase().contains("teste") || res_test.translation.to_lowercase().contains("exame"));

        // Everyday random English words
        for (w, expected_part) in [
            ("window", "janela"),
            ("apple", "maçã"),
            ("building", "edifício"),
            ("banana", "banana"),
            ("freedom", "liberdade"),
            ("whisper", "cochichar"),
            ("walnut", "noz"),
            ("connector", "conector"),
            ("shadow", "sombra"),
            ("elephant", "elefante"),
        ] {
            let res = dict.lookup(w).unwrap_or_else(|| panic!("Expected '{}' to be found", w));
            assert!(
                res.translation.to_lowercase().contains(expected_part)
                || res.definition.to_lowercase().contains(expected_part),
                "For word '{}', expected '{}' in translation '{}'", w, expected_part, res.translation
            );
        }

        // Irregulars & Conjugations
        assert!(dict.lookup("was").is_some(), "'was' should resolve to 'be'");
        assert!(dict.lookup("were").is_some(), "'were' should resolve to 'be'");
        assert!(dict.lookup("gotten").is_some(), "'gotten' should resolve to 'get'");
        assert!(dict.lookup("men").is_some(), "'men' should resolve to 'man'");
        assert!(dict.lookup("women").is_some(), "'women' should resolve to 'woman'");
        assert!(dict.lookup("replied").is_some(), "'replied' should resolve to 'reply'");
        assert!(dict.lookup("knives").is_some(), "'knives' should resolve to 'knife'");
        assert!(dict.lookup("running").is_some(), "'running' should resolve to 'run'");
        assert!(dict.lookup("easily").is_some(), "'easily' should resolve to 'easy'");
        assert!(dict.lookup("darkness").is_some(), "'darkness' should resolve to 'dark'");

        // Gaming terms
        let res_flank = dict.lookup("flank").expect("'flank' should exist");
        assert_eq!(res_flank.normalized, "flank");
        assert!(res_flank.translation.to_lowercase().contains("flanquear"));

        let res_sniper = dict.lookup("sniper").expect("'sniper' should exist");
        assert!(res_sniper.translation.to_lowercase().contains("atirador de elite"));

        let res_reload = dict.lookup("reload").expect("'reload' should exist");
        assert!(res_reload.translation.to_lowercase().contains("recarregar"));

        let res_headshot = dict.lookup("headshot").expect("'headshot' should exist");
        assert!(res_headshot.translation.to_lowercase().contains("tiro na cabeça"));

        // Plural lemmatization
        let res_plural = dict.lookup("flanks").expect("'flanks' should exist");
        assert_eq!(res_plural.normalized, "flank");

        // Nonexistent
        let res_missing = dict.lookup("nonexistentword123xyz99");
        assert!(res_missing.is_none());
    }

    #[test]
    fn test_enriched_gaming_terms() {
        let dict = Dictionary::in_memory().unwrap();

        let terms_to_check = [
            "flank", "jungle", "peel", "aggro", "kite", "gank", "buff", "nerf",
            "clutch", "ace", "eco", "drop", "ping", "lag", "hitbox", "tilt",
            "smurf", "push", "split push", "aoe", "dot", "cc", "crowd control",
            "ult", "ultimate", "cooldown", "carry", "support", "tank", "dps",
            "bait", "choke point", "respawn", "spawn kill", "minion", "creep",
            "tower dive", "build", "loadout", "proc", "stacking", "leash",
            "loot", "farming", "freeze", "roam", "inting", "throw",
        ];

        for &term in &terms_to_check {
            let res = dict.lookup(term).unwrap_or_else(|| panic!("Expected gaming term '{}' to be found", term));
            assert_eq!(res.normalized, term.to_lowercase());
            assert!(!res.translation.is_empty(), "Translation for '{}' must not be empty", term);
            assert!(
                res.part_of_speech.as_deref() == Some("termo gamer") || res.part_of_speech.as_deref() == Some("gíria gamer"),
                "POS for '{}' was expected to be 'termo gamer' or 'gíria gamer', got {:?}", term, res.part_of_speech
            );
            assert!(res.definition.len() > 20, "Definition for '{}' should be rich and explanatory, got '{}'", term, res.definition);
            assert!(res.example.is_some(), "Example for '{}' should be present", term);
        }

        // Specific context checks to ensure gaming meaning takes precedence over literal
        let peel = dict.lookup("peel").unwrap();
        assert!(peel.definition.contains("proteger um aliado") || peel.definition.contains("afastando"));
        assert!(!peel.translation.contains("descascar"));

        let clutch = dict.lookup("clutch").unwrap();
        assert!(clutch.definition.contains("pressão") || clutch.definition.contains("desvantagem"));
        assert!(!clutch.translation.contains("embreagem"));

        let eco = dict.lookup("eco").unwrap();
        assert!(eco.translation.contains("econômica") || eco.definition.contains("guardando dinheiro"));

        let farming = dict.lookup("farming").unwrap();
        assert!(farming.definition.contains("ouro") || farming.definition.contains("recursos"));
        assert!(!farming.translation.contains("agricultura"));

        let tank = dict.lookup("tank").unwrap();
        assert!(tank.definition.contains("combates") || tank.definition.contains("absorver dano"));
        assert!(!tank.translation.contains("cisterna"));

        let ult = dict.lookup("ult").unwrap();
        assert!(ult.definition.contains("habilidade mais forte") || ult.definition.contains("recarga"));
    }
}

