mod common;

use std::collections::HashMap;

use zbobr_dispatcher::{ChecklistItem, Model, Parameter, Signal, Stage, Tool};

#[tokio::test]
async fn create_read_modify_task() {
    let setup = common::create_test_setup();
    let backend = &setup.backend;

    // -- Create a task with all optional fields populated --
    let mut params = HashMap::new();
    params.insert(Parameter::DestinationRepository, "owner/repo".to_string());
    params.insert(Parameter::DestinationBranch, "main".to_string());

    let id = backend
        .create_task(
            "Test task title",
            "Test task description",
            Stage::Pending,
            Some(Tool::Claude),
            Some(Model::ClaudeOpus4_6),
            params,
        )
        .await
        .expect("create_task failed");

    assert_eq!(id, 1, "first task should get id 1");

    // -- Read the task and verify all fields --
    let task = backend.get_task(id).await.expect("get_task failed");

    assert_eq!(task.id, id);
    assert_eq!(task.title, "Test task title");
    assert_eq!(task.description, "Test task description");
    assert_eq!(task.plan, "");
    assert_eq!(task.stage, Stage::Pending);
    assert_eq!(task.tool, Some(Tool::Claude));
    assert_eq!(task.model, Some(Model::ClaudeOpus4_6));
    assert_eq!(
        task.parameters.get(&Parameter::DestinationRepository),
        Some(&"owner/repo".to_string()),
    );
    assert_eq!(
        task.parameters.get(&Parameter::DestinationBranch),
        Some(&"main".to_string()),
    );
    assert!(!task.conflict);
    assert!(!task.pause);
    assert!(task.checklist.is_empty());
    assert!(task.signal.is_none());

    // -- Modify the task: update multiple fields --
    backend
        .modify_task(
            id,
            Box::new(|mut t| {
                t.title = "Updated title".to_string();
                t.description = "Updated description".to_string();
                t.plan = "Step 1: do stuff\nStep 2: profit".to_string();
                t.stage = Stage::Working;
                t.conflict = true;
                t.pause = true;
                t.signal = Some(Signal::GoReview);
                t.checklist = vec![
                    ChecklistItem {
                        id: "item-1".to_string(),
                        checked: true,
                        text: "Write code".to_string(),
                    },
                    ChecklistItem {
                        id: "item-2".to_string(),
                        checked: false,
                        text: "Write tests".to_string(),
                    },
                ];
                t.parameters
                    .insert(Parameter::WorkBranch, "zbobr-1-fix".to_string());
                t
            }),
        )
        .await
        .expect("modify_task failed");

    // -- Re-read and verify modifications persisted --
    let task = backend
        .get_task(id)
        .await
        .expect("get_task after modify failed");

    assert_eq!(task.title, "Updated title");
    assert_eq!(task.description, "Updated description");
    assert_eq!(task.plan, "Step 1: do stuff\nStep 2: profit");
    assert_eq!(task.stage, Stage::Working);
    assert!(task.conflict);
    assert!(task.pause);
    assert_eq!(task.signal, Some(Signal::GoReview));
    assert_eq!(task.checklist.len(), 2);
    assert_eq!(task.checklist[0].id, "item-1");
    assert!(task.checklist[0].checked);
    assert_eq!(task.checklist[0].text, "Write code");
    assert_eq!(task.checklist[1].id, "item-2");
    assert!(!task.checklist[1].checked);
    assert_eq!(task.checklist[1].text, "Write tests");
    assert_eq!(
        task.parameters.get(&Parameter::WorkBranch),
        Some(&"zbobr-1-fix".to_string()),
    );
    // Original parameters should still be present
    assert_eq!(
        task.parameters.get(&Parameter::DestinationRepository),
        Some(&"owner/repo".to_string()),
    );

    // -- Close the task and verify --
    assert!(
        !backend
            .is_task_closed(id)
            .await
            .expect("is_task_closed failed")
    );

    backend.close_task(id).await.expect("close_task failed");

    assert!(
        backend
            .is_task_closed(id)
            .await
            .expect("is_task_closed after close failed")
    );

    // Closed tasks should not appear in list_tasks_by_stage
    let working_tasks = backend
        .list_tasks_by_stage(Stage::Working, None)
        .await
        .expect("list_tasks_by_stage failed");
    assert!(
        working_tasks.is_empty(),
        "closed task should not appear in stage listing"
    );
}
