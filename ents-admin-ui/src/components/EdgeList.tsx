import { Link } from 'react-router-dom'
import type { EdgeInfo } from '../api/types'

interface EdgeListProps {
  edges: EdgeInfo[]
  direction: 'outgoing' | 'incoming'
  loading?: boolean
}

export default function EdgeList({ edges, direction, loading }: EdgeListProps) {
  if (loading) {
    return (
      <div className="text-gray-400 text-sm py-4 text-center">
        Loading edges...
      </div>
    )
  }

  if (edges.length === 0) {
    return (
      <div className="text-gray-500 text-sm py-4 text-center">
        No {direction} edges
      </div>
    )
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-700">
            <th className="text-left py-2 px-3 text-gray-400 font-medium">
              {direction === 'outgoing' ? 'Source' : 'Source'}
            </th>
            <th className="text-left py-2 px-3 text-gray-400 font-medium">Edge Name</th>
            <th className="text-left py-2 px-3 text-gray-400 font-medium">
              {direction === 'outgoing' ? 'Destination' : 'Destination'}
            </th>
          </tr>
        </thead>
        <tbody>
          {edges.map((edge, i) => (
            <tr key={i} className="border-b border-gray-800 hover:bg-gray-800/50">
              <td className="py-2 px-3">
                <Link
                  to={`/entities/${edge.source}`}
                  className="text-blue-400 hover:text-blue-300"
                >
                  {edge.source}
                </Link>
              </td>
              <td className="py-2 px-3 text-gray-300 font-mono">
                {edge.sort_key}
              </td>
              <td className="py-2 px-3">
                <Link
                  to={`/entities/${edge.dest}`}
                  className="text-blue-400 hover:text-blue-300"
                >
                  {edge.dest}
                </Link>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
