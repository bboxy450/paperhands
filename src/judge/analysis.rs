//! Fixed-fold evaluation: threshold selection and logistic calibration see training rows only.
use super::*;
use std::path::PathBuf;

#[derive(Clone, Copy, Serialize)]
struct Metrics {
    tp: usize,
    fp: usize,
    tn: usize,
    #[serde(rename = "fn")]
    fn_: usize,
    precision: Option<f64>,
    recall: Option<f64>,
    f_beta: Option<f64>,
}
fn metric(tp: usize, fp: usize, tn: usize, missed: usize, beta: f64) -> Metrics {
    let precision = (tp + fp > 0).then(|| tp as f64 / (tp + fp) as f64);
    let recall = (tp + missed > 0).then(|| tp as f64 / (tp + missed) as f64);
    let b = beta * beta;
    let denom = (1.0 + b) * tp as f64 + b * missed as f64 + fp as f64;
    Metrics {
        tp,
        fp,
        tn,
        fn_: missed,
        precision,
        recall,
        f_beta: (denom > 0.0).then(|| (1.0 + b) * tp as f64 / denom),
    }
}
fn measure(rows: &[(f64, bool)], threshold: f64, beta: f64) -> Metrics {
    let (mut tp, mut fp, mut tn, mut missed) = (0, 0, 0, 0);
    for &(p, y) in rows {
        match (p >= threshold, y) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, false) => tn += 1,
            (false, true) => missed += 1,
        }
    }
    metric(tp, fp, tn, missed, beta)
}
fn meets(m: Metrics, precision: f64, recall: f64) -> bool {
    m.precision.is_some_and(|p| p >= precision) && m.recall.is_some_and(|r| r >= recall)
}
fn tune(rows: &[(f64, bool)], precision: f64, recall: f64, beta: f64) -> Option<f64> {
    let mut rows = rows.to_vec();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let positives = rows.iter().filter(|r| r.1).count();
    let negatives = rows.len() - positives;
    let (mut tp, mut fp, mut at) = (0, 0, 0);
    let mut best: Option<(f64, Metrics)> = None;
    while at < rows.len() {
        let threshold = rows[at].0;
        while at < rows.len() && rows[at].0 == threshold {
            if rows[at].1 {
                tp += 1
            } else {
                fp += 1
            };
            at += 1;
        }
        let m = metric(tp, fp, negatives - fp, positives - tp, beta);
        if meets(m, precision, recall)
            && best.is_none_or(|(t, b)| {
                m.f_beta > b.f_beta
                    || (m.f_beta == b.f_beta
                        && (m.precision > b.precision
                            || (m.precision == b.precision && threshold > t)))
            })
        {
            best = Some((threshold, m));
        }
    }
    best.map(|(t, _)| t)
}
fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-6, 1.0 - 1e-6);
    p.ln() - (-p).ln_1p()
}
fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}
fn loss(rows: &[(f64, bool)], a: f64, b: f64) -> f64 {
    rows.iter()
        .map(|&(p, y)| {
            let z = a * logit(p) + b;
            z.max(0.0) + (-z.abs()).exp().ln_1p() - if y { z } else { 0.0 }
        })
        .sum::<f64>()
        + 0.5e-6 * (a * a + b * b)
}
fn fit(rows: &[(f64, bool)]) -> Option<(f64, f64)> {
    let positives = rows.iter().filter(|r| r.1).count();
    if positives == 0 || positives == rows.len() {
        return None;
    }
    let (mut a, mut b) = (0.0, logit(positives as f64 / rows.len() as f64));
    for _ in 0..100 {
        let (mut ga, mut gb, mut haa, mut hab, mut hbb) = (1e-6 * a, 1e-6 * b, 1e-6, 0.0, 1e-6);
        for &(p, y) in rows {
            let x = logit(p);
            let q = sigmoid(a * x + b);
            let err = q - if y { 1.0 } else { 0.0 };
            let h = q * (1.0 - q);
            ga += err * x;
            gb += err;
            haa += h * x * x;
            hab += h * x;
            hbb += h;
        }
        if ga.abs().max(gb.abs()) < 1e-7 {
            return Some((a, b));
        }
        let det = haa * hbb - hab * hab;
        if !det.is_finite() || det <= 0.0 {
            return None;
        }
        let (da, db) = ((hbb * ga - hab * gb) / det, (haa * gb - hab * ga) / det);
        let old = loss(rows, a, b);
        let mut step = 1.0;
        let mut accepted = false;
        for _ in 0..30 {
            let (na, nb) = (a - step * da, b - step * db);
            if na.is_finite() && nb.is_finite() && loss(rows, na, nb) <= old {
                a = na;
                b = nb;
                accepted = true;
                break;
            }
            step *= 0.5;
        }
        if !accepted {
            return None;
        }
    }
    None // A failed fit is not silently applied.
}
fn brier(rows: &[(f64, bool)]) -> f64 {
    rows.iter()
        .map(|(p, y)| (p - if *y { 1.0 } else { 0.0 }).powi(2))
        .sum::<f64>()
        / rows.len() as f64
}
fn reliability(rows: &[(f64, bool)]) -> Vec<Value> {
    let mut rows = rows.to_vec();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Equal-count bins, with the actual group counts retained.
    rows.chunks(rows.len().div_ceil(5).max(1)).map(|bin|json!({"n":bin.len(),"mean_score":bin.iter().map(|r|r.0).sum::<f64>()/bin.len() as f64,"observed_failure_rate":bin.iter().filter(|r|r.1).count() as f64/bin.len() as f64})).collect()
}
fn cost(saved: &Scores) -> Option<f64> {
    let p = saved.pricing.as_ref()?;
    let (mut input, mut output) = (0u64, 0u64);
    for row in &saved.cases {
        let u = row.usage.as_ref()?;
        input = input.checked_add(u.input_tokens)?;
        output = output.checked_add(u.output_tokens)?;
    }
    let result = (input as f64 * p.input_usd_per_million
        + output as f64 * p.output_usd_per_million)
        / 1_000_000.0;
    result.is_finite().then_some(result)
}
fn model(data: &Dataset, saved: &Scores, min_p: f64, min_r: f64, beta: f64) -> (Value, Value) {
    let lookup = saved
        .cases
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect::<BTreeMap<_, _>>();
    let gaps=data.cases.iter().filter(|c|c.failure.is_none() || lookup[c.id.as_str()].score.is_none()).map(|c|json!({"id":c.id,"label_missing":c.failure.is_none(),"score_status":lookup[c.id.as_str()].status})).collect::<Vec<_>>();
    let estimated = cost(saved);
    if !gaps.is_empty() {
        return (
            json!({"id":saved.id,"provider":saved.provider,"model":saved.model,"status":"unsupported_incomplete_cases","gaps":gaps,"validation_eligible":false,"estimated_cost_usd":estimated,"pricing":saved.pricing}),
            json!([]),
        );
    }
    let all = data
        .cases
        .iter()
        .map(|c| (lookup[c.id.as_str()].score.unwrap(), c.failure.unwrap()))
        .collect::<Vec<_>>();
    let mut folds = Vec::new();
    let mut predictions = Vec::new();
    let mut tuned = Vec::new();
    let mut calibrated = Vec::new();
    let (mut all_thresholds, mut all_fits, mut every_fold_meets) = (true, true, true);
    for fold in 0..data.folds {
        let train = data
            .cases
            .iter()
            .zip(&all)
            .filter(|(c, _)| c.fold != fold)
            .map(|(_, r)| *r)
            .collect::<Vec<_>>();
        let test = data
            .cases
            .iter()
            .zip(&all)
            .filter(|(c, _)| c.fold == fold)
            .collect::<Vec<_>>();
        let valid = test.iter().map(|(_, r)| **r).collect::<Vec<_>>();
        let both = |rows: &[(f64, bool)]| rows.iter().any(|r| r.1) && rows.iter().any(|r| !r.1);
        let supported = both(&train);
        let threshold = supported
            .then(|| tune(&train, min_p, min_r, beta))
            .flatten();
        let fit = supported.then(|| fit(&train)).flatten();
        let validation = threshold.map(|t| measure(&valid, t, beta));
        let passes = both(&valid) && validation.is_some_and(|m| meets(m, min_p, min_r));
        all_thresholds &= threshold.is_some();
        all_fits &= fit.is_some();
        every_fold_meets &= passes;
        let ids = |test: bool| {
            data.cases
                .iter()
                .filter(|c| (c.fold == fold) == test)
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>()
        };
        folds.push(json!({"fold":fold,"training_n":train.len(),"validation_n":valid.len(),"training_case_ids_sha256":inv::hash(&serde_json::to_vec(&ids(false)).unwrap()),"validation_case_ids_sha256":inv::hash(&serde_json::to_vec(&ids(true)).unwrap()),"both_classes_in_training":supported,"both_classes_in_validation":both(&valid),"training_selected_threshold":threshold,"no_qualifying_threshold":threshold.is_none(),"validation":validation,"validation_meets_minimums":passes,"default_validation":measure(&valid,0.5,beta),"platt_fit":fit.map(|(a,b)|json!({"slope":a,"intercept":b})),"calibration_fit_supported":fit.is_some()}));
        for (case, &(p, y)) in test {
            let calibrated_score = fit.map(|(a, b)| sigmoid(a * logit(p) + b));
            if let Some(q) = calibrated_score {
                calibrated.push((q, y));
            }
            if let Some(t) = threshold {
                tuned.push((if p >= t { 1.0 } else { 0.0 }, y));
            }
            predictions.push(json!({"id":case.id,"group":case.group,"fold":fold,"failure":y,"raw_score":p,"training_selected_threshold":threshold,"predicted_failure":threshold.map(|t|p>=t),"platt_score":calibrated_score}));
        }
    }
    let raw_brier = brier(&all);
    let calibrated_brier = all_fits.then(|| brier(&calibrated));
    let calibrated_bins = all_fits.then(|| reliability(&calibrated));
    let validation = all_thresholds.then(|| measure(&tuned, 0.5, beta));
    let eligible = every_fold_meets && validation.is_some_and(|m| meets(m, min_p, min_r));
    let curve = (0..=100)
        .map(|i| {
            let threshold = i as f64 / 100.0;
            json!({"threshold":threshold,"metrics":measure(&all,threshold,beta)})
        })
        .collect::<Vec<_>>();
    (
        json!({"id":saved.id,"provider":saved.provider,"model":saved.model,"status":"evaluated","n":all.len(),"folds":folds,"default_validation":measure(&all,0.5,beta),"tuned_validation":validation,"all_training_folds_qualified":all_thresholds,"all_validation_folds_meet_minimums":every_fold_meets,"validation_eligible":eligible,
        "development_fit_threshold":tune(&all,min_p,min_r,beta),"development_threshold_curve":curve,
        "calibration":{"method":"ridge-logistic-on-clipped-log-odds","l2":1e-6,"clip":1e-6,"max_iterations":100,"all_training_fits_supported":all_fits,"raw_brier":raw_brier,"platt_validation_brier":calibrated_brier,"platt_improved":calibrated_brier.map(|b|b<raw_brier),"raw_reliability":reliability(&all),"platt_validation_reliability":calibrated_bins},
        "estimated_cost_usd":estimated,"pricing":saved.pricing,"cost_scope":"declared token rates times reported usage; null when missing; not an enforced billing cap",
        "measured_call_wall_ms":saved.cases.iter().try_fold(0u64,|sum,row|sum.checked_add(row.wall_time_ms?))}),
        json!(predictions),
    )
}

pub(super) fn analyze(
    data_path: &Path,
    score_paths: &[PathBuf],
    out: &Path,
    min_p: f64,
    min_r: f64,
    beta: f64,
) -> Result<Value> {
    if !min_p.is_finite()
        || !min_r.is_finite()
        || !(0.0..=1.0).contains(&min_p)
        || !(0.0..=1.0).contains(&min_r)
        || !beta.is_finite()
        || !(0.01..=100.0).contains(&beta)
        || score_paths.is_empty()
        || score_paths.len() > 16
    {
        return Err("analysis requires 1..16 saved score files, unit precision/recall and beta in 0.01..100".into());
    }
    let data_bytes = read_bytes(data_path)?;
    let data = dataset(&data_bytes)?;
    let data_hash = inv::hash(&data_bytes);
    let mut inputs = Vec::new();
    let mut rubric = None;
    let mut ids = BTreeSet::new();
    let mut total = 0;
    for path in score_paths {
        let bytes = read_bytes(path)?;
        total += bytes.len();
        if total > 64 * 1024 * 1024 {
            return Err("saved score inputs exceed 64 MiB".into());
        }
        let saved = scores(&bytes, &data, &data_hash)?;
        if rubric.as_ref().is_some_and(|r| r != &saved.rubric) {
            return Err("model comparison requires the same exact failure rubric".into());
        }
        if !ids.insert(saved.id.clone()) {
            return Err("duplicate saved judge identity in analysis".into());
        }
        rubric = Some(saved.rubric.clone());
        inputs.push((bytes, saved));
    }
    let id = out
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("analysis directory needs a UTF-8 name")?;
    inv::name(id)?;
    private_dir(out)?;
    let evidence = out.join("evidence");
    private_dir(&evidence)?;
    write(&evidence.join("dataset.json"), &data_bytes)?;
    write_json(
        &evidence.join("started.json"),
        &json!({"id":id,"kind":"analysis","engine_version":crate::VERSION,"started_at_ms":inv::now(),"pid":std::process::id()}),
    )?;
    let mut models = Vec::new();
    let mut sources = Vec::new();
    for (i, (bytes, saved)) in inputs.iter().enumerate() {
        let source = format!("scores-{i:02}.json");
        write(&evidence.join(&source), bytes)?;
        sources.push(json!({"id":saved.id,"file":source,"sha256":inv::hash(bytes)}));
        let (mut report, predictions) = model(&data, saved, min_p, min_r, beta);
        let file = format!("predictions-{i:02}.json");
        write_json(&evidence.join(&file), &predictions)?;
        report["predictions"] = json!({"file":file,"sha256":inv::digest(&evidence.join(&file))?});
        models.push(report);
    }
    let frontier = models
        .iter()
        .filter(|m| m["validation_eligible"] == true && m["estimated_cost_usd"].is_number())
        .filter(|m| {
            let (cost, quality) = (
                m["estimated_cost_usd"].as_f64().unwrap(),
                m["tuned_validation"]["f_beta"].as_f64().unwrap(),
            );
            !models.iter().any(|other| {
                other["id"] != m["id"]
                    && other["validation_eligible"] == true
                    && other["estimated_cost_usd"].as_f64().is_some_and(|c| {
                        c <= cost
                            && other["tuned_validation"]["f_beta"]
                                .as_f64()
                                .is_some_and(|q| q >= quality && (c < cost || q > quality))
                    })
            })
        })
        .map(|m| m["id"].clone())
        .collect::<Vec<_>>();
    let supported = models.iter().filter(|m| m["status"] == "evaluated").count();
    let report = json!({"schema":"paperhands.judge-analysis/v1","engine_version":crate::VERSION,"kind":"analysis","id":id,"finished_at_ms":inv::now(),"dataset_sha256":data_hash,"source_data_use":data.data_use,"data_use_after_analysis":"development","rubric":rubric,"sources":sources,
        "protocol":{"folds":data.folds,"fold_assignment":"caller-supplied and frozen; related groups cannot cross folds","threshold_operator":">=","objective":"F-beta","beta":beta,"min_precision":min_p,"min_recall":min_r,"tie_break":"F-beta, precision, then higher threshold","no_qualifying_threshold":"null; never relax constraints or substitute 0.50","calibration_fit":"training fold only; compare raw and out-of-fold recalibrated probabilities","validation_eligibility":"every validation fold must meet the declared minimums","model_calls":0},
        "models":models,"cost_quality_frontier":frontier,"supported_models":supported,"exit_code":if supported==inputs.len(){0}else{3},"actions_executed":false,
        "limitations":["Imported scores, labels, grouping and causal information boundaries are caller-supplied; hashes do not validate them.","Fixed grouped folds do not provide purged, embargoed or forward-time evaluation; dependent market data may require those separate splits.","Cross-validation uses every labeled row for development. This is not untouched holdout or new forward evidence.","Selecting prompts/models on these results adds selection bias; confirm the chosen protocol on fresh evidence.","Reliability bins and Brier loss are descriptive at this sample size; Brier also reflects discrimination, not calibration alone.","No score/threshold/declared cost grants execution, financial validity, research agenda control or strategy promotion."]});
    write_json(&evidence.join("result.json"), &report)?;
    let mut markdown = String::from(
        "# Decision-model eval\n\nOffline grouped-fold development analysis. No new model calls or trading actions.\n\n| Judge | Status | Validation eligible | Raw Brier | Calibrated Brier | Estimated USD |\n| --- | --- | --- | --- | --- | --- |\n",
    );
    for m in report["models"].as_array().unwrap() {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            m["id"].as_str().unwrap(),
            m["status"].as_str().unwrap(),
            m["validation_eligible"],
            m["calibration"]["raw_brier"],
            m["calibration"]["platt_validation_brier"],
            m["estimated_cost_usd"]
        ));
    }
    markdown.push_str("\nNull is unsupported or unknown, not zero. See result.json for folds, constraint failures, fixed inputs, threshold curves, calibration and selection limits. Raw cases and labels remain private. This summary is also private; it is not a financial publication artifact.\n");
    write(&evidence.join("report.md"), markdown.as_bytes())?;
    seal(out)?;
    Ok(
        json!({"dir":inv::root(out)?,"integrity_status":"verified","exit_code":report["exit_code"],"result":report}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threshold_tuning_is_constrained_and_calibration_is_measured() {
        let rows = vec![(0.1, false), (0.1, false), (0.4, true), (0.4, true)];
        assert_eq!(tune(&rows, 0.8, 0.8, 1.0), Some(0.4));
        assert_eq!(measure(&rows, 0.5, 1.0).recall, Some(0.0));
        assert_eq!(tune(&[(0.8, false), (0.8, true)], 0.8, 0.8, 1.0), None);
        let (a, b) = fit(&rows).unwrap();
        let calibrated = rows
            .iter()
            .map(|&(p, y)| (sigmoid(a * logit(p) + b), y))
            .collect::<Vec<_>>();
        assert!(brier(&calibrated) < brier(&rows));
        assert!(fit(&[(0.5, true), (0.9, true)]).is_none());
        assert!(logit(0.0).is_finite() && logit(1.0).is_finite());
    }

    #[test]
    fn validation_labels_cannot_change_the_training_threshold_or_mapping() {
        let mut data:Dataset=serde_json::from_value(json!({"schema":"paperhands.judge-dataset/v1","id":"leakage","data_use":"synthetic","folds":2,"cases":[
            {"id":"a0","group":"a","fold":0,"text":"x","failure":false},{"id":"a1","group":"a","fold":0,"text":"x","failure":true},
            {"id":"b0","group":"b","fold":1,"text":"x","failure":false},{"id":"b1","group":"b","fold":1,"text":"x","failure":true}]})).unwrap();
        let mut saved = Scores {
            schema: "paperhands.judge-scores/v1".into(),
            id: "low".into(),
            dataset_sha256: "unit-only".into(),
            provider: "synthetic".into(),
            model: "toy".into(),
            rubric: Rubric {
                id: "source".into(),
                failure: "unsupported source?".into(),
            },
            pricing: None,
            cases: data
                .cases
                .iter()
                .map(|c| Score {
                    id: c.id.clone(),
                    status: "scored".into(),
                    score: Some(if c.failure.unwrap() { 0.4 } else { 0.1 }),
                    usage: None,
                    wall_time_ms: None,
                    error: None,
                })
                .collect(),
        };
        let before = model(&data, &saved, 0.8, 0.8, 1.0).0;
        for c in &mut data.cases {
            if c.fold == 0 {
                c.failure = Some(true);
            }
        }
        let after = model(&data, &saved, 0.8, 0.8, 1.0).0;
        assert_eq!(
            before["folds"][0]["training_selected_threshold"],
            json!(0.4)
        );
        assert_eq!(
            before["folds"][0]["training_selected_threshold"],
            after["folds"][0]["training_selected_threshold"]
        );
        assert_eq!(
            before["folds"][0]["platt_fit"],
            after["folds"][0]["platt_fit"]
        );
        assert_eq!(after["folds"][0]["both_classes_in_validation"], false);
        // A shifted mechanism can make recalibration worse. Retain that result.
        data.cases[0].failure = Some(false);
        for (i, row) in saved.cases.iter_mut().enumerate() {
            row.score = Some(if i < 2 {
                if i % 2 == 0 { 0.1 } else { 0.9 }
            } else if i % 2 == 0 {
                0.9
            } else {
                0.1
            });
        }
        let shifted = model(&data, &saved, 0.8, 0.8, 1.0).0;
        assert_eq!(shifted["calibration"]["platt_improved"], false);
        assert!(
            shifted["calibration"]["platt_validation_brier"]
                .as_f64()
                .unwrap()
                > shifted["calibration"]["raw_brier"].as_f64().unwrap()
        );
    }
}
