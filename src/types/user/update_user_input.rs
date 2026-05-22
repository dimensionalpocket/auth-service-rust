/// High-level input type for updateUser mutation
///
/// This type represents application-level input data that is:
/// - Created by the resolver from GraphQL arguments
/// - Passed to the orchestrator for validation and business logic
/// - Not used directly in GraphQL schema (schema uses individual arguments)
///
/// This establishes a clean pattern: resolver creates app-level types,
/// orchestrator accepts app-level types, service layer performs business logic.
#[derive(Debug)]
pub struct UpdateUserInput {
  pub id: i64,
  pub name: Option<String>,
  pub role_id: Option<i64>,
  pub password: Option<String>,
  pub password_confirmation: Option<String>,
  pub metadata_json: Option<String>,
}
