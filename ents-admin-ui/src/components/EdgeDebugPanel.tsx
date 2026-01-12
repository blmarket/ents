import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { auditEntityEdges, fixEntityEdges } from '../api'
import type { AuditResult, EdgeInfo } from '../api/types'

interface EdgeDebugPanelProps {
  entityId: number
  entityType: string
}

const KNOWN_TYPES = ['TestEntity', 'User', 'Post', 'Tag']

export default function EdgeDebugPanel({ entityId, entityType }: EdgeDebugPanelProps) {
  const [selectedType, setSelectedType] = useState(entityType || KNOWN_TYPES[0])
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null)

  const auditMutation = useMutation({
    mutationFn: () => auditEntityEdges(entityId, selectedType),
    onSuccess: (result) => setAuditResult(result),
  })

  const fixMutation = useMutation({
    mutationFn: () => fixEntityEdges(entityId, selectedType),
    onSuccess: () => {
      auditMutation.mutate()
    },
  })

  const renderEdgeList = (edges: EdgeInfo[], label: string) => {
    if (edges.length === 0) return null
    return (
      <div className="mt-2">
        <div className="text-xs text-gray-400 mb-1">{label}:</div>
        <div className="space-y-1">
          {edges.map((edge, i) => (
            <div key={i} className="text-xs font-mono bg-gray-800 px-2 py-1 rounded">
              {edge.source} --[{edge.sort_key}]--&gt; {edge.dest}
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="border border-gray-700 rounded">
      <div className="bg-gray-800 px-3 py-2 border-b border-gray-700">
        <h3 className="text-sm font-medium text-gray-300">Edge Audit</h3>
      </div>
      <div className="p-3">
        <div className="flex items-center gap-2 mb-3">
          <select
            value={selectedType}
            onChange={(e) => setSelectedType(e.target.value)}
            className="px-2 py-1 text-sm bg-gray-700 border border-gray-600 rounded text-white"
          >
            {KNOWN_TYPES.map((t) => (
              <option key={t} value={t}>{t}</option>
            ))}
          </select>
          <button
            onClick={() => auditMutation.mutate()}
            disabled={auditMutation.isPending}
            className="px-3 py-1 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors disabled:opacity-50"
          >
            {auditMutation.isPending ? 'Auditing...' : 'Audit Edges'}
          </button>
          {auditResult && !auditResult.valid && (
            <button
              onClick={() => {
                if (confirm('This will repair mismatched edges. Continue?')) {
                  fixMutation.mutate()
                }
              }}
              disabled={fixMutation.isPending}
              className="px-3 py-1 text-sm bg-yellow-600 hover:bg-yellow-700 text-white rounded transition-colors disabled:opacity-50"
            >
              {fixMutation.isPending ? 'Fixing...' : 'Fix Edges'}
            </button>
          )}
        </div>

        {auditMutation.error && (
          <div className="bg-red-900/30 border border-red-800 rounded px-3 py-2 text-sm text-red-400 mb-3">
            {(auditMutation.error as Error).message}
          </div>
        )}

        {auditResult && (
          <div className={`rounded px-3 py-2 ${auditResult.valid ? 'bg-green-900/30 border border-green-800' : 'bg-yellow-900/30 border border-yellow-800'}`}>
            <div className="flex items-center gap-2">
              {auditResult.valid ? (
                <>
                  <span className="text-green-400">&#10003;</span>
                  <span className="text-sm text-green-400">Edges are valid</span>
                </>
              ) : (
                <>
                  <span className="text-yellow-400">&#9888;</span>
                  <span className="text-sm text-yellow-400">Edge mismatch detected</span>
                </>
              )}
            </div>
            {!auditResult.valid && (
              <div className="mt-2 space-y-2">
                {renderEdgeList(auditResult.missing, 'Missing edges')}
                {renderEdgeList(auditResult.extra, 'Extra edges')}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
