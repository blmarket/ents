import { useState } from 'react'
import { useNavigate } from 'react-router-dom'

export default function HomePage() {
  const [entityId, setEntityId] = useState('')
  const navigate = useNavigate()

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const id = parseInt(entityId, 10)
    if (!isNaN(id) && id > 0) {
      navigate(`/entities/${id}`)
    }
  }

  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh]">
      <h2 className="text-2xl font-semibold text-gray-200 mb-6">
        Entity Browser
      </h2>
      <p className="text-gray-400 mb-8 text-center max-w-md">
        Enter an entity ID to view its data, edges, and perform audit operations.
      </p>
      <form onSubmit={handleSubmit} className="w-full max-w-sm">
        <div className="flex flex-col gap-3">
          <input
            type="text"
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            placeholder="Entity ID (e.g., 1)"
            className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
            autoFocus
          />
          <button
            type="submit"
            className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
          >
            View Entity
          </button>
        </div>
      </form>
    </div>
  )
}
