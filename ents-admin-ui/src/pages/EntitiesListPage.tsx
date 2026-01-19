import { useState, useEffect } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { getKnownTypes, listEntities, createEntity } from '../api/client'
import type { Entity } from '../api/types'
import EntityCreator from '../components/EntityCreator'

export function EntitiesListPage() {
  const navigate = useNavigate()
  const [entityTypes, setEntityTypes] = useState<string[]>([])
  const [selectedType, setSelectedType] = useState<string>('')
  const [entities, setEntities] = useState<Entity[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [cursor, setCursor] = useState<number | undefined>(undefined)
  const [hasMore, setHasMore] = useState(false)
  const [loadingTypes, setLoadingTypes] = useState(true)
  const [showCreator, setShowCreator] = useState(false)
  const [creatorType, setCreatorType] = useState<string>('')
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)

  const PAGE_SIZE = 95

  useEffect(() => {
    loadEntityTypes()
  }, [])

  useEffect(() => {
    if (selectedType) {
      loadEntities(true)
    }
  }, [selectedType])

  const loadEntityTypes = async () => {
    try {
      setLoadingTypes(true)
      const types = await getKnownTypes()
      setEntityTypes(types)
      if (types.length > 0 && !selectedType) {
        setSelectedType(types[0])
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load entity types')
    } finally {
      setLoadingTypes(false)
    }
  }

  const loadEntities = async (reset: boolean = false) => {
    if (!selectedType) return

    try {
      setLoading(true)
      setError(null)

      const newCursor = reset ? undefined : cursor
      const result = await listEntities(selectedType, PAGE_SIZE + 1, newCursor)

      // Check if there are more entities
      const hasMoreResults = result.length > PAGE_SIZE
      const displayEntities = hasMoreResults ? result.slice(0, PAGE_SIZE) : result

      if (reset) {
        setEntities(displayEntities)
      } else {
        setEntities(prev => [...prev, ...displayEntities])
      }

      setHasMore(hasMoreResults)
      if (displayEntities.length > 0) {
        setCursor(displayEntities[displayEntities.length - 1].id)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load entities')
    } finally {
      setLoading(false)
    }
  }

  const handleTypeChange = (type: string) => {
    setSelectedType(type)
    setEntities([])
    setCursor(undefined)
    setHasMore(false)
  }

  const loadMore = () => {
    loadEntities(false)
  }

  const openCreator = () => {
    setCreatorType(selectedType || entityTypes[0] || '')
    setCreateError(null)
    setShowCreator(true)
  }

  const handleCreate = async (entity: object) => {
    try {
      setCreating(true)
      setCreateError(null)
      const result = await createEntity(entity)
      setShowCreator(false)
      // Navigate to the newly created entity
      navigate(`/entities/${result.id}`)
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : 'Failed to create entity')
    } finally {
      setCreating(false)
    }
  }

  if (loadingTypes) {
    return (
      <div className="flex justify-center items-center min-h-[60vh]">
        <div className="text-gray-400">Loading entity types...</div>
      </div>
    )
  }

  return (
    <div className="max-w-6xl mx-auto p-6">
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-3xl font-bold text-white">Entity Browser</h1>
          <button
            onClick={openCreator}
            disabled={entityTypes.length === 0}
            className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded transition-colors flex items-center gap-2"
          >
            <span>+</span>
            <span>Create Entity</span>
          </button>
        </div>
        <p className="text-gray-400 mb-4">
          Browse and manage entities by type. Select an entity type to view all entities of that type.
        </p>

        {/* Type Selector */}
        <div className="flex flex-wrap gap-2 mb-4">
          {entityTypes.map(type => (
            <button
              key={type}
              onClick={() => handleTypeChange(type)}
              className={`px-4 py-2 rounded transition-colors ${
                selectedType === type
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {type}
            </button>
          ))}
        </div>

        {selectedType && (
          <div className="text-sm text-gray-400 mb-4">
            Showing entities of type: <span className="font-semibold text-white">{selectedType}</span>
          </div>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div className="bg-red-900/50 border border-red-700 text-red-200 p-4 rounded mb-4">
          {error}
        </div>
      )}

      {/* Entities List */}
      {selectedType && (
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          {entities.length === 0 && !loading ? (
            <div className="p-8 text-center text-gray-400">
              No entities found for type "{selectedType}"
            </div>
          ) : (
            <>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead className="bg-gray-700">
                    <tr>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                        ID
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                        Type
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                        Last Updated
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-700">
                    {entities.map(entity => (
                      <tr key={entity.id} className="hover:bg-gray-750">
                        <td className="px-4 py-3 text-sm text-gray-300">
                          {entity.id}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-300">
                          {entity.type}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-300">
                          {new Date(entity.last_updated * 1000).toLocaleString()}
                        </td>
                        <td className="px-4 py-3 text-sm">
                          <Link
                            to={`/entities/${entity.id}`}
                            className="text-blue-400 hover:text-blue-300 transition-colors"
                          >
                            View Details
                          </Link>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Load More Button */}
              {hasMore && (
                <div className="p-4 border-t border-gray-700 text-center">
                  <button
                    onClick={loadMore}
                    disabled={loading}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800 disabled:cursor-not-allowed text-white rounded transition-colors"
                  >
                    {loading ? 'Loading...' : 'Load More'}
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {/* Loading Indicator */}
      {loading && entities.length === 0 && (
        <div className="flex justify-center items-center py-8">
          <div className="text-gray-400">Loading entities...</div>
        </div>
      )}

      {/* Entity Creator Modal */}
      {showCreator && (
        <EntityCreator
          selectedType={creatorType}
          entityTypes={entityTypes}
          onTypeChange={setCreatorType}
          onCreate={handleCreate}
          onCancel={() => setShowCreator(false)}
          creating={creating}
          error={createError}
        />
      )}
    </div>
  )
}
