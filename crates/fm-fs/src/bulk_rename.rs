//! Executor assíncrono em background de planos de renomeação em lote e desfazer seguro.

use std::collections::HashSet;
use std::path::PathBuf;

use fm_core::{
    BulkRenameEvaluation, BulkRenameExecutionPlan, BulkRenamePreviewRow, BulkRenameRowStatus,
    BulkRenameTargetPolicy, EntryId, FsError, RenameStep, build_execution_plan,
};

/// Resultado detalhado da execução de um lote de renomeações.
#[derive(Debug, Clone)]
pub struct BulkRenameBatchOutcome {
    pub renamed_count: usize,
    pub failed: Option<(PathBuf, String)>,
    pub rolled_back_temps: usize,
    /// Pares `(caminho_atual, caminho_anterior)` para desfazer com segurança na sessão.
    pub undo_pairs: Vec<(PathBuf, PathBuf)>,
    /// Caminhos resultantes bem-sucedidos para manter seleção na aba após refresh.
    pub resulting_paths: Vec<PathBuf>,
}

/// Executa o plano de renomeação fora da thread de UI com proteção contra sobrescrita e rollback.
pub async fn execute_bulk_rename_plan(
    plan: BulkRenameExecutionPlan,
) -> BulkRenameBatchOutcome {
    tokio::task::spawn_blocking(move || execute_plan_blocking(&plan))
        .await
        .unwrap_or_else(|err| BulkRenameBatchOutcome {
            renamed_count: 0,
            failed: Some((PathBuf::new(), err.to_string())),
            rolled_back_temps: 0,
            undo_pairs: Vec::new(),
            resulting_paths: Vec::new(),
        })
}

/// Desfaz um lote concluído reutilizando o planejador topológico/2-fases sobre os pares reversos.
pub async fn undo_bulk_rename_pairs(
    undo_pairs: Vec<(PathBuf, PathBuf)>,
    batch_id: u64,
    policy: BulkRenameTargetPolicy,
) -> Result<usize, FsError> {
    if undo_pairs.is_empty() {
        return Ok(0);
    }

    tokio::task::spawn_blocking(move || {
        let mut rows = Vec::with_capacity(undo_pairs.len());
        let existing_paths = HashSet::new();

        for (idx, (current_path, original_path)) in undo_pairs.iter().enumerate() {
            if crate::archive::split_archive_path(current_path).is_some()
                || crate::archive::split_archive_path(original_path).is_some()
            {
                return Err(FsError::Io {
                    path: current_path.clone(),
                    message: "arquivos compactados são somente leitura".to_owned(),
                });
            }
            let Some(parent_dir) = current_path.parent().map(PathBuf::from) else {
                continue;
            };
            let Some(orig_name) = current_path.file_name().map(std::ffi::OsStr::to_os_string)
            else {
                continue;
            };
            let Some(new_name) = original_path.file_name().map(std::ffi::OsStr::to_os_string)
            else {
                continue;
            };

            rows.push(BulkRenamePreviewRow {
                input_index: idx,
                sequence_pos: idx + 1,
                entry_id: EntryId(u64::try_from(idx + 1).unwrap_or(1)),
                src_path: current_path.clone(),
                parent_dir,
                original_name: orig_name,
                new_name,
                dst_path: original_path.clone(),
                is_dir: false,
                status: BulkRenameRowStatus::Ready,
            });
        }

        let eval = BulkRenameEvaluation {
            ready_count: rows.len(),
            rows,
            regex_error: None,
            unchanged_count: 0,
            skipped_count: 0,
            conflict_count: 0,
            invalid_count: 0,
        };

        let Some(plan) = build_execution_plan(&eval, &existing_paths, batch_id, policy) else {
            return Err(FsError::Io {
                path: undo_pairs[0].0.clone(),
                message: "não foi possível montar o plano reverso de renomeação".to_owned(),
            });
        };

        let outcome = execute_plan_blocking(&plan);
        if let Some((path, message)) = outcome.failed {
            return Err(FsError::Io { path, message });
        }
        Ok(outcome.renamed_count)
    })
    .await
    .map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: err.to_string(),
    })?
}

fn execute_plan_blocking(plan: &BulkRenameExecutionPlan) -> BulkRenameBatchOutcome {
    let mut undo_pairs = Vec::with_capacity(plan.total_renames());
    let mut resulting_paths = Vec::with_capacity(plan.total_renames());
    let mut renamed_count = 0usize;

    // Fase A: passos diretos já ordenados topologicamente.
    for step in &plan.direct_steps {
        if let Some(err) = check_archive_or_target_collision(step) {
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err)),
                rolled_back_temps: 0,
                undo_pairs,
                resulting_paths,
            };
        }
        if let Err(err) = std::fs::rename(&step.from, &step.to) {
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err.to_string())),
                rolled_back_temps: 0,
                undo_pairs,
                resulting_paths,
            };
        }
        renamed_count += 1;
        resulting_paths.push(step.to.clone());
        undo_pairs.push((step.to.clone(), step.from.clone()));
    }

    // Fase 1: mover itens em ciclo ou case-only para nomes temporários únicos.
    let mut completed_temps = 0usize;
    for step in &plan.phase1_temp_steps {
        if let Some(err) = check_archive_or_target_collision(step) {
            let rolled = rollback_pending_temps(&plan.rollback_temp_steps[..completed_temps]);
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err)),
                rolled_back_temps: rolled,
                undo_pairs,
                resulting_paths,
            };
        }
        if let Err(err) = std::fs::rename(&step.from, &step.to) {
            let rolled = rollback_pending_temps(&plan.rollback_temp_steps[..completed_temps]);
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err.to_string())),
                rolled_back_temps: rolled,
                undo_pairs,
                resulting_paths,
            };
        }
        completed_temps += 1;
    }

    // Fase 2: mover temporários para os destinos finais.
    for (idx, step) in plan.phase2_final_steps.iter().enumerate() {
        if let Some(err) = check_archive_or_target_collision(step) {
            let rolled = rollback_pending_temps(&plan.rollback_temp_steps[idx..completed_temps]);
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err)),
                rolled_back_temps: rolled,
                undo_pairs,
                resulting_paths,
            };
        }
        if let Err(err) = std::fs::rename(&step.from, &step.to) {
            let rolled = rollback_pending_temps(&plan.rollback_temp_steps[idx..completed_temps]);
            return BulkRenameBatchOutcome {
                renamed_count,
                failed: Some((step.from.clone(), err.to_string())),
                rolled_back_temps: rolled,
                undo_pairs,
                resulting_paths,
            };
        }
        renamed_count += 1;
        resulting_paths.push(step.to.clone());
        let orig_src = &plan.phase1_temp_steps[idx].from;
        undo_pairs.push((step.to.clone(), orig_src.clone()));
    }

    BulkRenameBatchOutcome {
        renamed_count,
        failed: None,
        rolled_back_temps: 0,
        undo_pairs,
        resulting_paths,
    }
}

fn check_archive_or_target_collision(step: &RenameStep) -> Option<String> {
    if crate::archive::split_archive_path(&step.from).is_some()
        || crate::archive::split_archive_path(&step.to).is_some()
    {
        return Some("arquivos compactados são somente leitura".to_owned());
    }
    if std::fs::symlink_metadata(&step.to).is_ok() {
        return Some(format!(
            "destino já existe no disco: {}",
            step.to.to_string_lossy()
        ));
    }
    None
}

fn rollback_pending_temps(steps: &[RenameStep]) -> usize {
    let mut restored = 0usize;
    for step in steps {
        if std::fs::symlink_metadata(&step.from).is_ok()
            && std::fs::symlink_metadata(&step.to).is_err()
            && std::fs::rename(&step.from, &step.to).is_ok()
        {
            restored += 1;
        }
    }
    restored
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::{BulkRenameInput, BulkRenameRules, MatchScope, evaluate_bulk_rename};
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vfm_fs_br_test_{nanos}_{label}"));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    fn run_async<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(fut)
    }

    #[test]
    fn executes_swap_cycle_and_undo_without_data_loss() {
        let dir = temp_dir("swap");
        let a_path = dir.join("01.txt");
        let b_path = dir.join("02.txt");
        std::fs::write(&a_path, b"alpha").expect("write a");
        std::fs::write(&b_path, b"beta").expect("write b");

        let eval = BulkRenameEvaluation {
            rows: vec![
                BulkRenamePreviewRow {
                    input_index: 0,
                    sequence_pos: 1,
                    entry_id: EntryId(1),
                    src_path: a_path.clone(),
                    parent_dir: dir.clone(),
                    original_name: OsString::from("01.txt"),
                    new_name: OsString::from("02.txt"),
                    dst_path: b_path.clone(),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
                BulkRenamePreviewRow {
                    input_index: 1,
                    sequence_pos: 2,
                    entry_id: EntryId(2),
                    src_path: b_path.clone(),
                    parent_dir: dir.clone(),
                    original_name: OsString::from("02.txt"),
                    new_name: OsString::from("01.txt"),
                    dst_path: a_path.clone(),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
            ],
            regex_error: None,
            ready_count: 2,
            unchanged_count: 0,
            skipped_count: 0,
            conflict_count: 0,
            invalid_count: 0,
        };

        let plan = build_execution_plan(
            &eval,
            &HashSet::new(),
            42,
            BulkRenameTargetPolicy::host_default(),
        )
        .expect("plan");
        let outcome = run_async(execute_bulk_rename_plan(plan));
        assert!(outcome.failed.is_none());
        assert_eq!(outcome.renamed_count, 2);
        assert_eq!(std::fs::read(&a_path).expect("read a"), b"beta");
        assert_eq!(std::fs::read(&b_path).expect("read b"), b"alpha");

        // Desfazendo a troca de volta para o original
        let undone = run_async(undo_bulk_rename_pairs(
            outcome.undo_pairs,
            43,
            BulkRenameTargetPolicy::host_default(),
        ))
        .expect("undo ok");
        assert_eq!(undone, 2);
        assert_eq!(std::fs::read(&a_path).expect("read a"), b"alpha");
        assert_eq!(std::fs::read(&b_path).expect("read b"), b"beta");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn executes_chain_and_aborts_on_concurrent_collision() {
        let dir = temp_dir("chain");
        let f1 = dir.join("01.txt");
        let f2 = dir.join("02.txt");
        let f3 = dir.join("03.txt");
        std::fs::write(&f1, b"first").expect("write 1");
        std::fs::write(&f2, b"second").expect("write 2");

        let inputs = [
            BulkRenameInput {
                entry_id: EntryId(1),
                path: f1.clone(),
                parent_dir: dir.clone(),
                original_name: OsString::from("01.txt"),
                is_dir: false,
                size_bytes: 5,
                modified_secs: None,
            },
            BulkRenameInput {
                entry_id: EntryId(2),
                path: f2.clone(),
                parent_dir: dir.clone(),
                original_name: OsString::from("02.txt"),
                is_dir: false,
                size_bytes: 6,
                modified_secs: None,
            },
        ];

        // 01 -> 02, 02 -> 03 via evaluate + plan manual? Melhor testar colisão direta com 01->02 e 02->03:
        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &BulkRenameRules {
                replace_enabled: true,
                find_text: "01".to_owned(),
                replace_with: "03".to_owned(),
                replace_scope: MatchScope::StartOfName,
                ..Default::default()
            },
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        let plan = build_execution_plan(
            &eval,
            &HashSet::new(),
            9,
            BulkRenameTargetPolicy::host_default(),
        )
        .expect("plan");

        // Simula criação concorrente de `03.txt` antes de rodar o plano
        std::fs::write(&f3, b"external").expect("write 3");
        let outcome = run_async(execute_bulk_rename_plan(plan));
        assert!(outcome.failed.is_some());
        assert_eq!(std::fs::read(&f3).expect("read 3"), b"external");
        assert!(f1.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
