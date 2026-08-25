use super::super::types::SettingsAllConfigImportError;
use super::contracts::{DefaultImportPolicy, DefaultStateRow};
use std::collections::HashMap;

pub(in crate::config::settings_all) fn ensure_single_default_after_import<T>(
    kind: &str,
    existing: &[DefaultStateRow],
    incoming: &[T],
    overwrite: bool,
    policy: DefaultImportPolicy,
    name: impl Fn(&T) -> &str,
    is_default: impl Fn(&T) -> bool,
) -> Result<(), SettingsAllConfigImportError> {
    let mut result = existing.to_vec();
    let mut first_index_by_name = HashMap::with_capacity(result.len() + incoming.len());
    for (index, row) in result.iter().enumerate() {
        first_index_by_name.entry(row.name.clone()).or_insert(index);
    }
    let existing_default_count = result.iter().filter(|row| row.is_default).count();
    let incoming_default_count = incoming.iter().filter(|row| is_default(*row)).count();
    if incoming_default_count > 1 {
        return Err(SettingsAllConfigImportError::InvalidArgument(format!(
            "全量导入文件只能声明一个{kind}，当前为 {incoming_default_count} 个"
        )));
    }

    for row in incoming {
        let row_name = name(row);
        let row_is_default = is_default(row);
        let applied_index = match first_index_by_name.get(row_name).copied() {
            Some(index) if overwrite => {
                result[index].is_default = row_is_default;
                Some(index)
            }
            Some(_) => None,
            None => {
                result.push(DefaultStateRow {
                    name: row_name.to_string(),
                    is_default: row_is_default,
                });
                let index = result.len() - 1;
                first_index_by_name.insert(row_name.to_string(), index);
                Some(index)
            }
        };

        if let Some(default_index) = applied_index.filter(|_| row_is_default) {
            for (index, item) in result.iter_mut().enumerate() {
                item.is_default = index == default_index;
            }
        }
    }

    let default_count = result.iter().filter(|row| row.is_default).count();
    match default_count {
        1 => Ok(()),
        0 if policy == DefaultImportPolicy::PreserveIfConfigured
            && existing_default_count == 0
            && incoming_default_count == 0 =>
        {
            Ok(())
        }
        0 => Err(SettingsAllConfigImportError::InvalidArgument(format!(
            "全量导入后必须保留一个{kind}，请在导入文件中设置一个默认项或先设置数据库默认项"
        ))),
        count => Err(SettingsAllConfigImportError::InvalidArgument(format!(
            "全量导入后只能保留一个{kind}，当前为 {count} 个"
        ))),
    }
}
