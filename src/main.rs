use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::Path;

use serde::Deserialize;

const RESULTS_LIMIT: usize = 20;
const EXCLUDED_DESCRIPTION_TERMS: [&str; 42] = [
    "cereal",
    "smoothie",
    "dressing",
    "mollusks",
    "babyfood",
    "chrysanthemum",
    "margarine",
    "syrup",
    "spcies",
    "spices",
    "tofu",
    "frozen",
    "cooked",
    "formulated",
    "snackfood",
    "fortified",
    "canned",
    "sauce",
    "yeast",
    "liver",
    "tablets",
    "oil",
    "beverage",
    "shortening",
    "butter",
    "snack",
    "formula",
    "infant",
    "candies",
    "topping",
    "cocoa",
    "soup",
    "luncheon",
    "burger",
    "flour",
    "bacon",
    "pasta",
    "toddler",
    "crustaceans",
    "mushrooms",
    "gelatins",
    "gelatin",
];

const NUTRIENTS: [NutrientSpec; 21] = [
    NutrientSpec::new("Vitamin A", "µg", &["vitamin-a", "a"], &["320"]),
    NutrientSpec::new(
        "Vitamin B1 (Thiamine)",
        "mg",
        &["vitamin-b1", "b1", "thiamine"],
        &["404"],
    ),
    NutrientSpec::new(
        "Vitamin B2 (Riboflavin)",
        "mg",
        &["vitamin-b2", "b2", "riboflavin"],
        &["405"],
    ),
    NutrientSpec::new(
        "Vitamin B3 (Niacin)",
        "mg",
        &["vitamin-b3", "b3", "niacin"],
        &["406"],
    ),
    NutrientSpec::new(
        "Vitamin B5 (Pantothenic Acid)",
        "mg",
        &["vitamin-b5", "b5", "pantothenic-acid", "pantothenic"],
        &["410"],
    ),
    NutrientSpec::new(
        "Vitamin B6 (Pyridoxine)",
        "mg",
        &["vitamin-b6", "b6", "pyridoxine"],
        &["415"],
    ),
    NutrientSpec::new(
        "Vitamin B12 (Cobalamin)",
        "µg",
        &["vitamin-b12", "b12", "cobalamin"],
        &["418"],
    ),
    NutrientSpec::new("Folate", "µg", &["folate", "vitamin-b9", "b9"], &["417"]),
    NutrientSpec::new("Vitamin C", "mg", &["vitamin-c", "c"], &["401"]),
    NutrientSpec::new("Vitamin E", "mg", &["vitamin-e", "e"], &["323"]),
    NutrientSpec::new("Vitamin K", "µg", &["vitamin-k", "k"], &["430"]),
    NutrientSpec::new("Calcium", "mg", &["calcium"], &["301"]),
    NutrientSpec::new("Copper", "mg", &["copper"], &["312"]),
    NutrientSpec::new("Iron", "mg", &["iron"], &["303"]),
    NutrientSpec::new("Magnesium", "mg", &["magnesium"], &["304"]),
    NutrientSpec::new("Manganese", "mg", &["manganese"], &["315"]),
    NutrientSpec::new("Phosphorus", "mg", &["phosphorus"], &["305"]),
    NutrientSpec::new("Potassium", "mg", &["potassium"], &["306"]),
    NutrientSpec::new("Selenium", "µg", &["selenium"], &["317"]),
    NutrientSpec::new("Sodium", "mg", &["sodium"], &["307"]),
    NutrientSpec::new("Zinc", "mg", &["zinc"], &["309"]),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_args(&args) {
        Ok(Command::Help) => print_help(),
        Ok(Command::ListNutrients) => print_nutrients(),
        Ok(Command::DirectLookup { nutrient, metric }) => match load_data() {
            Ok(data) => print_rankings(&data, nutrient, metric),
            Err(error) => exit_with_error(&error),
        },
        Ok(Command::MetricMenu(metric)) => match load_data() {
            Ok(data) => run_nutrient_menu(&data, metric),
            Err(error) => exit_with_error(&error),
        },
        Ok(Command::Interactive) => match load_data() {
            Ok(data) => run_main_menu(&data),
            Err(error) => exit_with_error(&error),
        },
        Err(error) => {
            eprintln!("{error}");
            eprintln!();
            print_help();
            std::process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Interactive);
    }

    let mut nutrient_name: Option<&str> = None;
    let mut metric: Option<Metric> = None;

    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--nutrient" | "--nutrients" | "--nurient" => return Ok(Command::ListNutrients),
            "--per-calorie" => set_metric(&mut metric, Metric::PerCalorie)?,
            "--per-gram" => set_metric(&mut metric, Metric::PerGram)?,
            value if value.starts_with('-') => {
                return Err(format!("Unknown flag: {value}"));
            }
            value => {
                if nutrient_name.replace(value).is_some() {
                    return Err("Please provide only one nutrient name.".to_string());
                }
            }
        }
    }

    match (nutrient_name, metric) {
        (Some(name), Some(metric)) => {
            let nutrient = find_nutrient(name).ok_or_else(|| {
                format!("Unknown nutrient: {name}. Use --nutrient to list valid names.")
            })?;
            Ok(Command::DirectLookup { nutrient, metric })
        }
        (None, Some(metric)) => Ok(Command::MetricMenu(metric)),
        (Some(_), None) => Err(
            "Please include either --per-calorie or --per-gram with the nutrient name.".to_string(),
        ),
        (None, None) => Ok(Command::Interactive),
    }
}

fn set_metric(current: &mut Option<Metric>, next: Metric) -> Result<(), String> {
    if current.replace(next).is_some() {
        Err("Please provide only one metric flag.".to_string())
    } else {
        Ok(())
    }
}

fn print_help() {
    println!("nutrient-rank");
    println!();
    println!("Usage:");
    println!("  nutrient-rank");
    println!("  nutrient-rank --per-calorie");
    println!("  nutrient-rank --per-gram");
    println!("  nutrient-rank <nutrient> --per-calorie");
    println!("  nutrient-rank <nutrient> --per-gram");
    println!("  nutrient-rank --nutrient");
    println!("  nutrient-rank --help");
    println!();
    println!("Examples:");
    println!("  nutrient-rank iron --per-calorie");
    println!("  nutrient-rank iron --per-gram");
    println!("  nutrient-rank --per-calorie");
    println!("  nutrient-rank --per-gram");
    println!();
    println!("Notes:");
    println!("  --nutrient, --nutrients, and --nurient all print the available nutrient names.");
    println!("  Running without arguments opens the interactive menu.");
}

fn print_nutrients() {
    println!("Available nutrients:");
    for nutrient in NUTRIENTS {
        println!("- {}", nutrient.display_name);
    }
}

fn run_main_menu(data: &AppData) {
    loop {
        println!();
        println!("Main Menu");
        println!("1. {}", Metric::PerCalorie.menu_label());
        println!("2. {}", Metric::PerGram.menu_label());
        println!("3. Nutrient Target Within Weight Limit");
        println!("0. Exit");

        match prompt_for_selection("Choose an option: ", 3) {
            Some(0) => {
                println!("Goodbye.");
                break;
            }
            Some(1) => run_nutrient_menu(data, Metric::PerCalorie),
            Some(2) => run_nutrient_menu(data, Metric::PerGram),
            Some(3) => run_target_menu(data),
            Some(_) => println!("Please enter a valid number from the menu."),
            None => println!("Please enter a valid number from the menu."),
        }
    }
}

fn run_nutrient_menu(data: &AppData, metric: Metric) {
    loop {
        println!();
        println!("{}", metric.menu_label());
        for (index, nutrient) in NUTRIENTS.iter().enumerate() {
            println!("{}. {}", index + 1, nutrient.display_name);
        }
        println!("0. Exit");

        match prompt_for_selection("Choose a nutrient: ", NUTRIENTS.len()) {
            Some(0) => break,
            Some(selection) => {
                println!();
                print_rankings(data, NUTRIENTS[selection - 1], metric);
                wait_for_enter();
                break;
            }
            None => println!("Please enter a valid number from the menu."),
        }
    }
}

fn print_rankings(data: &AppData, nutrient: NutrientSpec, metric: Metric) {
    let rankings = rank_foods(data, nutrient, metric);

    println!(
        "Top {} foods for {} {}",
        RESULTS_LIMIT,
        nutrient.display_name,
        metric.flag_label()
    );
    println!();

    if rankings.is_empty() {
        println!("No matching foods were found.");
        return;
    }

    for (index, entry) in rankings.iter().take(RESULTS_LIMIT).enumerate() {
        println!(
            "{:>2}. {} [{}] - {:.6} {}",
            index + 1,
            entry.description,
            entry.source,
            entry.score,
            metric.score_unit(&entry.nutrient_unit)
        );
    }
}

fn run_target_menu(data: &AppData) {
    loop {
        println!();
        println!("Nutrient Target Within Weight Limit");
        for (index, nutrient) in NUTRIENTS.iter().enumerate() {
            println!("{}. {}", index + 1, nutrient.display_name);
        }
        println!("0. Exit");

        let nutrient = match prompt_for_selection("Choose a nutrient: ", NUTRIENTS.len()) {
            Some(0) => break,
            Some(selection) => NUTRIENTS[selection - 1],
            None => {
                println!("Please enter a valid number from the menu.");
                continue;
            }
        };

        let target_amount = match prompt_for_positive_f64(&format!(
            "Enter the amount of {} you need ({}): ",
            nutrient.display_name, nutrient.default_unit
        )) {
            Some(value) => value,
            None => {
                println!("Please enter a number greater than 0.");
                continue;
            }
        };

        let max_weight_grams = match prompt_for_positive_f64("Enter the weight constraint (g): ") {
            Some(value) => value,
            None => {
                println!("Please enter a number greater than 0.");
                continue;
            }
        };

        println!();
        print_target_results(data, nutrient, target_amount, max_weight_grams);
        wait_for_enter();
        break;
    }
}

fn print_target_results(
    data: &AppData,
    nutrient: NutrientSpec,
    target_amount: f64,
    max_weight_grams: f64,
) {
    let candidates = rank_target_candidates(data, nutrient, target_amount, max_weight_grams);

    println!(
        "Top {} foods for {} target of {:.3} {} within {:.3} g",
        RESULTS_LIMIT,
        nutrient.display_name,
        target_amount,
        nutrient.default_unit,
        max_weight_grams
    );
    println!();

    if candidates.is_empty() {
        println!("No foods can meet that target within the weight limit.");
        return;
    }

    for (index, candidate) in candidates.iter().take(RESULTS_LIMIT).enumerate() {
        println!(
            "{:>2}. {} [{}] - {:.2} g needed, {:.2} kcal, {:.3} {}/100g",
            index + 1,
            candidate.description,
            candidate.source,
            candidate.required_grams,
            candidate.required_calories,
            candidate.nutrient_per_100g,
            candidate.nutrient_unit
        );
    }
}

fn rank_target_candidates(
    data: &AppData,
    nutrient: NutrientSpec,
    target_amount: f64,
    max_weight_grams: f64,
) -> Vec<TargetCandidate> {
    let mut candidates = Vec::new();

    for food in data
        .foundation_foods
        .iter()
        .map(|food| ("Foundation", food))
        .chain(data.sr_legacy_foods.iter().map(|food| ("SR Legacy", food)))
    {
        if let Some(candidate) =
            score_target_candidate(food.0, food.1, nutrient, target_amount, max_weight_grams)
        {
            candidates.push(candidate);
        }
    }

    candidates.sort_by(|left, right| {
        left.required_calories
            .partial_cmp(&right.required_calories)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                left.required_grams
                    .partial_cmp(&right.required_grams)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.description.cmp(&right.description))
    });
    candidates
}

fn score_target_candidate(
    source: &'static str,
    food: &Food,
    nutrient: NutrientSpec,
    target_amount: f64,
    max_weight_grams: f64,
) -> Option<TargetCandidate> {
    let nutrient_match = food.food_nutrients.iter().find(|entry| {
        nutrient
            .nutrient_numbers
            .contains(&entry.nutrient.number.as_str())
    })?;

    let nutrient_per_100g = nutrient_match.amount?;
    if nutrient_per_100g <= 0.0 {
        return None;
    }

    let required_grams = target_amount / (nutrient_per_100g / 100.0);
    if !required_grams.is_finite() || required_grams <= 0.0 || required_grams > max_weight_grams {
        return None;
    }

    let calories_per_100g = calories_for_food(food)?;
    if calories_per_100g < 0.0 {
        return None;
    }

    let required_calories = calories_per_100g * (required_grams / 100.0);
    if !required_calories.is_finite() {
        return None;
    }

    Some(TargetCandidate {
        description: food.description.clone(),
        source,
        required_grams,
        required_calories,
        nutrient_per_100g,
        nutrient_unit: nutrient_match.nutrient.unit_name.clone(),
    })
}

fn rank_foods(data: &AppData, nutrient: NutrientSpec, metric: Metric) -> Vec<RankedFood> {
    let mut rankings = Vec::new();

    for food in data
        .foundation_foods
        .iter()
        .map(|food| ("Foundation", food))
        .chain(data.sr_legacy_foods.iter().map(|food| ("SR Legacy", food)))
    {
        if let Some(entry) = score_food(food.0, food.1, nutrient, metric) {
            rankings.push(entry);
        }
    }

    rankings.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.description.cmp(&right.description))
    });
    rankings
}

fn score_food(
    source: &'static str,
    food: &Food,
    nutrient: NutrientSpec,
    metric: Metric,
) -> Option<RankedFood> {
    let nutrient_match = food.food_nutrients.iter().find(|entry| {
        nutrient
            .nutrient_numbers
            .contains(&entry.nutrient.number.as_str())
    })?;

    let nutrient_amount = nutrient_match.amount?;
    if nutrient_amount <= 0.0 {
        return None;
    }

    let score = match metric {
        Metric::PerGram => nutrient_amount / 100.0,
        Metric::PerCalorie => {
            let calories = calories_for_food(food)?;
            if calories <= 0.0 {
                return None;
            }
            nutrient_amount / calories
        }
    };

    if !score.is_finite() || score <= 0.0 {
        return None;
    }

    Some(RankedFood {
        description: food.description.clone(),
        source,
        score,
        nutrient_unit: nutrient_match.nutrient.unit_name.clone(),
    })
}

fn calories_for_food(food: &Food) -> Option<f64> {
    food.food_nutrients
        .iter()
        .find(|entry| entry.nutrient.number == "208")
        .and_then(|entry| entry.amount)
}

fn prompt_for_selection(prompt: &str, max_option: usize) -> Option<usize> {
    print!("{prompt}");
    io::stdout().flush().ok()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;

    let selection = input.trim().parse::<usize>().ok()?;
    if selection <= max_option {
        Some(selection)
    } else {
        None
    }
}

fn prompt_for_positive_f64(prompt: &str) -> Option<f64> {
    print!("{prompt}");
    io::stdout().flush().ok()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;

    let value = input.trim().parse::<f64>().ok()?;
    if value > 0.0 { Some(value) } else { None }
}

fn wait_for_enter() {
    println!();
    print!("Press Enter to continue...");
    let _ = io::stdout().flush();

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

fn load_data() -> Result<AppData, String> {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    let foundation_path = data_dir.join("FoodData_Central_foundation_food_json_2025-12-18.json");
    let sr_legacy_path = data_dir.join("FoodData_Central_sr_legacy_food_json_2018-04.json");

    let foundation: FoundationRoot = read_json_file(&foundation_path)?;
    let sr_legacy: SrLegacyRoot = read_json_file(&sr_legacy_path)?;

    Ok(AppData {
        foundation_foods: foundation
            .foundation_foods
            .into_iter()
            .filter(|food| !should_exclude_food(food))
            .collect(),
        sr_legacy_foods: sr_legacy
            .sr_legacy_foods
            .into_iter()
            .filter(|food| !should_exclude_food(food))
            .collect(),
    })
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let file = File::open(path).map_err(|error| format!("{}: {}", path.display(), error))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|error| format!("{}: {}", path.display(), error))
}

fn find_nutrient(input: &str) -> Option<NutrientSpec> {
    let normalized = normalize_arg(input);
    NUTRIENTS.iter().copied().find(|nutrient| {
        nutrient.display_name.eq_ignore_ascii_case(input)
            || nutrient
                .aliases
                .iter()
                .any(|alias| normalize_arg(alias) == normalized)
    })
}

fn should_exclude_food(food: &Food) -> bool {
    let description = food.description.to_ascii_lowercase();
    EXCLUDED_DESCRIPTION_TERMS
        .iter()
        .any(|term| description.contains(term))
}

fn normalize_arg(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn exit_with_error(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

#[derive(Clone, Copy)]
enum Metric {
    PerCalorie,
    PerGram,
}

impl Metric {
    fn menu_label(self) -> &'static str {
        match self {
            Metric::PerCalorie => "Nutrient / Calorie",
            Metric::PerGram => "Nutrient / Gram",
        }
    }

    fn flag_label(self) -> &'static str {
        match self {
            Metric::PerCalorie => "per calorie",
            Metric::PerGram => "per gram",
        }
    }

    fn score_unit(self, nutrient_unit: &str) -> String {
        match self {
            Metric::PerCalorie => format!("{nutrient_unit}/kcal"),
            Metric::PerGram => format!("{nutrient_unit}/g"),
        }
    }
}

#[derive(Clone, Copy)]
struct NutrientSpec {
    display_name: &'static str,
    default_unit: &'static str,
    aliases: &'static [&'static str],
    nutrient_numbers: &'static [&'static str],
}

impl NutrientSpec {
    const fn new(
        display_name: &'static str,
        default_unit: &'static str,
        aliases: &'static [&'static str],
        nutrient_numbers: &'static [&'static str],
    ) -> Self {
        Self {
            display_name,
            default_unit,
            aliases,
            nutrient_numbers,
        }
    }
}

enum Command {
    Help,
    ListNutrients,
    DirectLookup {
        nutrient: NutrientSpec,
        metric: Metric,
    },
    MetricMenu(Metric),
    Interactive,
}

struct AppData {
    foundation_foods: Vec<Food>,
    sr_legacy_foods: Vec<Food>,
}

struct RankedFood {
    description: String,
    source: &'static str,
    score: f64,
    nutrient_unit: String,
}

struct TargetCandidate {
    description: String,
    source: &'static str,
    required_grams: f64,
    required_calories: f64,
    nutrient_per_100g: f64,
    nutrient_unit: String,
}

#[derive(Deserialize)]
struct FoundationRoot {
    #[serde(rename = "FoundationFoods")]
    foundation_foods: Vec<Food>,
}

#[derive(Deserialize)]
struct SrLegacyRoot {
    #[serde(rename = "SRLegacyFoods")]
    sr_legacy_foods: Vec<Food>,
}

#[derive(Deserialize)]
struct Food {
    description: String,
    #[serde(rename = "foodNutrients")]
    food_nutrients: Vec<FoodNutrient>,
}

#[derive(Deserialize)]
struct FoodNutrient {
    amount: Option<f64>,
    nutrient: Nutrient,
}

#[derive(Deserialize)]
struct Nutrient {
    number: String,
    #[serde(rename = "unitName")]
    unit_name: String,
}
