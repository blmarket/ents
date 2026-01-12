import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getEntity, getEntityEdges, getIncomingEdges, updateEntity, deleteEntity } from '../api'
import EntityViewer from '../components/EntityViewer'
import EntityEditor from '../components/EntityEditor'
import EdgeList from '../components/EdgeList'
import EdgeDebugPanel from '../components/EdgeDebugPanel'
import type { Entity } from '../api/types'

export default function EntityPage() {
  const { id } = useParams<{ id: string }>()
  const entityId = parseInt(id || '0', 10)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [isEditing, setIsEditing] = useState(false)
  const [activeTab, setActiveTab] = useState<'outgoing' | 'incoming' | 'audit'>('outgoing')

  const entityQuery = useQuery({
    queryKey: ['entity', entityId],
    queryFn: () => getEntity(entityId),
    enabled: entityId > 0,
  })

  const outgoingEdgesQuery = useQuery({
    queryKey: ['edges', entityId, 'outgoing'],
    queryFn: () => getEntityEdges(entityId),
    enabled: entityId > 0,
  })

  const incomingEdgesQuery = useQuery({
    queryKey: ['edges', entityId, 'incoming'],
    queryFn: () => getIncomingEdges(entityId),
    enabled: entityId > 0,
  })

  const updateMutation = useMutation({
    mutationFn: (entity: Entity) => updateEntity(entityId, entity),
    onSuccess: (updated) => {
      queryClient.setQueryData(['entity', entityId], updated)
      setIsEditing(false)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: () => deleteEntity(entityId),
    onSuccess: () => {
      navigate('/')
    },
  })

  if (!entityId || entityId <= 0) {
    return (
      <div className="text-center py-8 text-gray-400">
        Invalid entity ID
      </div>
    )
  }

  if (entityQuery.isLoading) {
    return (
      <div className="text-center py-8 text-gray-400">
        Loading entity...
      </div>
    )
  }

  if (entityQuery.error) {
    return (
      <div className="text-center py-8">
        <div className="text-red-400 mb-4">
          Error: {(entityQuery.error as Error).message}
        </div>
        <button
          onClick={() => navigate('/')}
          className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors"
        >
          Go Back
        </button>
      </div>
    )
  }

  const entity = entityQuery.data!

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-200">
          Entity #{entityId}
        </h2>
        <div className="flex gap-2">
          {!isEditing && (
            <>
              <button
                onClick={() => setIsEditing(true)}
                className="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
              >
                Edit
              </button>
              <button
                onClick={() => {
                  if (confirm('Are you sure you want to delete this entity?')) {
                    deleteMutation.mutate()
                  }
                }}
                disabled={deleteMutation.isPending}
                className="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-700 text-white rounded transition-colors disabled:opacity-50"
              >
                {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
              </button>
            </>
          )}
        </div>
      </div>

      {updateMutation.error && (
        <div className="bg-red-900/30 border border-red-800 rounded px-4 py-3 text-red-400">
          Error: {(updateMutation.error as Error).message}
        </div>
      )}

      {isEditing ? (
        <EntityEditor
          entity={entity}
          onSave={async (e) => {
            await updateMutation.mutateAsync(e)
          }}
          onCancel={() => setIsEditing(false)}
          saving={updateMutation.isPending}
        />
      ) : (
        <EntityViewer entity={entity} />
      )}

      <div className="border border-gray-700 rounded">
        <div className="bg-gray-800 border-b border-gray-700 flex">
          <button
            onClick={() => setActiveTab('outgoing')}
            className={`px-4 py-2 text-sm ${activeTab === 'outgoing' ? 'text-white bg-gray-700' : 'text-gray-400 hover:text-gray-300'}`}
          >
            Outgoing Edges ({outgoingEdgesQuery.data?.length || 0})
          </button>
          <button
            onClick={() => setActiveTab('incoming')}
            className={`px-4 py-2 text-sm ${activeTab === 'incoming' ? 'text-white bg-gray-700' : 'text-gray-400 hover:text-gray-300'}`}
          >
            Incoming Edges ({incomingEdgesQuery.data?.length || 0})
          </button>
          <button
            onClick={() => setActiveTab('audit')}
            className={`px-4 py-2 text-sm ${activeTab === 'audit' ? 'text-white bg-gray-700' : 'text-gray-400 hover:text-gray-300'}`}
          >
            Audit
          </button>
        </div>
        <div className="p-3">
          {activeTab === 'outgoing' && (
            <EdgeList
              edges={outgoingEdgesQuery.data || []}
              direction="outgoing"
              loading={outgoingEdgesQuery.isLoading}
            />
          )}
          {activeTab === 'incoming' && (
            <EdgeList
              edges={incomingEdgesQuery.data || []}
              direction="incoming"
              loading={incomingEdgesQuery.isLoading}
            />
          )}
          {activeTab === 'audit' && (
            <EdgeDebugPanel entityId={entityId} entityType={entity.type} />
          )}
        </div>
      </div>
    </div>
  )
}
