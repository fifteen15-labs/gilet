//! Counts the contract shapes `OPEN_PROBLEMS` §1.5 says read as junk — wage 0
//! expiring 2 January 1900 — to see whether the year bound and the
//! zero-wage sentinel skip already retired the problem.
//!
//! ```text
//! cargo run --release --example junkdeals -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: junkdeals <save.fm>");
        std::process::exit(2);
    };
    let save = fm_save::Save::parse(&std::fs::read(&path)?)?;

    let mut contracts = 0usize;
    let mut era_1900 = 0usize;
    let mut pre_1950 = 0usize;
    let mut zero_wage_dated = 0usize;
    let mut zero_wage_undated = 0usize;
    for p in &save.people {
        if p.wage.is_some() || p.contract_until.is_some() {
            contracts += 1;
        }
        if let Some(d) = p.contract_until {
            if d.year == 1900 {
                era_1900 += 1;
            }
            if d.year < 1950 {
                pre_1950 += 1;
            }
        }
        match (p.wage, p.contract_until) {
            (Some(0), Some(_)) => zero_wage_dated += 1,
            (Some(0), None) => zero_wage_undated += 1,
            _ => {}
        }
    }
    println!("people with any contract field: {contracts}");
    println!("expiry in 1900:                 {era_1900}");
    println!("expiry before 1950:             {pre_1950}");
    println!("wage 0 with an expiry:          {zero_wage_dated}");
    println!("wage 0 with no expiry:          {zero_wage_undated}");
    Ok(())
}
