import { useState } from 'react'
import { Outlet, useNavigate } from 'react-router-dom'

export default function Layout() {
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
    <div className="min-h-screen flex flex-col">
      <header className="bg-gray-800 border-b border-gray-700 px-4 py-3">
        <div className="max-w-7xl mx-auto flex items-center gap-4">
          <h1
            className="text-xl font-semibold text-white cursor-pointer"
            onClick={() => navigate('/')}
          >
            Ents Admin
          </h1>
          <form onSubmit={handleSubmit} className="flex-1 max-w-md">
            <div className="flex">
              <input
                type="text"
                value={entityId}
                onChange={(e) => setEntityId(e.target.value)}
                placeholder="Enter entity ID..."
                className="flex-1 px-3 py-1.5 bg-gray-700 border border-gray-600 rounded-l text-sm text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
              />
              <button
                type="submit"
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-r transition-colors"
              >
                Go
              </button>
            </div>
          </form>
        </div>
      </header>
      <main className="flex-1 p-4">
        <div className="max-w-7xl mx-auto">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
