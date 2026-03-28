In `github.rs`, there is a test `apply_flag_change_adds_and_removes_confirm_label` (around line 1455) that tests the now-deleted `apply_flag_change` method. Remove this test entirely.

If there are other tests that assert on `flag:*` labels being present in issues, update or remove those too — flags are now parameters in the issue body, not labels.