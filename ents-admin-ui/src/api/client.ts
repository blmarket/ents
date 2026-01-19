import type { Entity, EdgeInfo, EdgesResponse, CreateResponse, AuditResult } from './types'

const API_BASE = import.meta.env.VITE_API_URL || ''

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const contentType = response.headers.get('content-type') || ''
    let errorMessage: string

    if (contentType.includes('application/json')) {
      const error = await response.json().catch(() => null)
      errorMessage = error?.error || error?.message || `HTTP ${response.status}: ${response.statusText}`
    } else {
      const text = await response.text().catch(() => '')
      errorMessage = text || `HTTP ${response.status}: ${response.statusText}`
    }

    throw new Error(errorMessage)
  }
  return response.json()
}

export async function getKnownTypes(): Promise<string[]> {
  const response = await fetch(`${API_BASE}/api/types`)
  return handleResponse<string[]>(response)
}

export async function getEntity(id: number): Promise<Entity> {
  const response = await fetch(`${API_BASE}/api/entities/${id}`)
  return handleResponse<Entity>(response)
}

export async function getEntityEdges(id: number, name?: string): Promise<EdgesResponse> {
  const params = name ? `?name=${encodeURIComponent(name)}` : ''
  const response = await fetch(`${API_BASE}/api/entities/${id}/edges${params}`)
  return handleResponse<EdgesResponse>(response)
}

export async function getIncomingEdges(id: number): Promise<EdgeInfo[]> {
  const response = await fetch(`${API_BASE}/api/entities/${id}/incoming-edges`)
  return handleResponse<EdgeInfo[]>(response)
}

export async function createEntity(entity: Partial<Entity>): Promise<CreateResponse> {
  const response = await fetch(`${API_BASE}/api/entities`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(entity),
  })
  return handleResponse<CreateResponse>(response)
}

export async function updateEntity(id: number, entity: Entity): Promise<Entity> {
  const response = await fetch(`${API_BASE}/api/entities/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(entity),
  })
  return handleResponse<Entity>(response)
}

export async function deleteEntity(id: number): Promise<void> {
  const response = await fetch(`${API_BASE}/api/entities/${id}`, {
    method: 'DELETE',
  })
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }))
    throw new Error(error.error || `HTTP ${response.status}`)
  }
}

export async function auditEntityEdges(id: number, typeName: string): Promise<AuditResult> {
  const response = await fetch(
    `${API_BASE}/api/entities/${id}/audit-edges?type=${encodeURIComponent(typeName)}`,
    { method: 'POST' }
  )
  return handleResponse<AuditResult>(response)
}

export async function fixEntityEdges(id: number, typeName: string): Promise<void> {
  const response = await fetch(
    `${API_BASE}/api/entities/${id}/fix-edges?type=${encodeURIComponent(typeName)}`,
    { method: 'POST' }
  )
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }))
    throw new Error(error.error || `HTTP ${response.status}`)
  }
}

export async function listEntities(
  entityType: string,
  limit?: number,
  cursor?: number
): Promise<Entity[]> {
  const params = new URLSearchParams()
  params.set('type', entityType)
  if (limit) params.set('limit', limit.toString())
  if (cursor) params.set('cursor', cursor.toString())

  const response = await fetch(`${API_BASE}/api/entities?${params.toString()}`)
  return handleResponse<Entity[]>(response)
}
