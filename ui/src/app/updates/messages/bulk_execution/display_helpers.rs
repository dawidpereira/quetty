/// Format bulk delete success message
pub fn format_bulk_delete_success_message(
    actually_deleted_count: usize,
    originally_selected_count: usize,
    queue_name: &str,
) -> String {
    let title = "Bulk Delete Complete";
    let not_found_count = originally_selected_count - actually_deleted_count;

    if not_found_count > 0 {
        // Some messages were not found/deleted
        format!(
            "{}\n\n✅ Successfully deleted {} out of {} selected message{} from {}\n📍 Action: Messages permanently removed\n\n⚠️  {} message{} could not be found (may have already been processed)",
            title,
            actually_deleted_count,
            originally_selected_count,
            if originally_selected_count == 1 {
                ""
            } else {
                "s"
            },
            queue_name,
            not_found_count,
            if not_found_count == 1 { "" } else { "s" }
        )
    } else {
        // All messages were found and deleted
        format!(
            "{}\n\n✅ Successfully deleted {} message{} from {}\n📍 Action: Messages permanently removed",
            title,
            actually_deleted_count,
            if actually_deleted_count == 1 { "" } else { "s" },
            queue_name
        )
    }
}
