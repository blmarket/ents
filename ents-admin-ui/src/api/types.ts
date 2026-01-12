export type Id = number

export interface Entity {
  type: string
  id: Id
  last_updated: number
  [key: string]: unknown
}

export interface EdgeInfo {
  source: Id
  sort_key: string
  sort_key_bytes: number[]
  dest: Id
}

export interface CreateResponse {
  id: Id
  entity: Entity
}

export interface AuditResult {
  valid: boolean
  existing_edges: EdgeInfo[]
  expected_edges: EdgeInfo[]
  missing: EdgeInfo[]
  extra: EdgeInfo[]
}

export interface ApiError {
  error: string
}
