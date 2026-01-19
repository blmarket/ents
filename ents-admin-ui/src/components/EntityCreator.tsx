import { useState, useEffect } from 'react'
import Editor from '@monaco-editor/react'

interface EntityCreatorProps {
  selectedType: string
  entityTypes: string[]
  onTypeChange: (type: string) => void
  onCreate: (entity: object) => Promise<void>
  onCancel: () => void
  creating?: boolean
  error?: string | null
}

export default function EntityCreator({
  selectedType,
  entityTypes,
  onTypeChange,
  onCreate,
  onCancel,
  creating,
  error,
}: EntityCreatorProps) {
  const [jsonValue, setJsonValue] = useState('')
  const [parseError, setParseError] = useState<string | null>(null)

  useEffect(() => {
    // Initialize with a template for the selected type
    const template = {
      type: selectedType,
    }
    setJsonValue(JSON.stringify(template, null, 2))
    setParseError(null)
  }, [selectedType])

  const handleChange = (value: string | undefined) => {
    const val = value || ''
    setJsonValue(val)
    try {
      JSON.parse(val)
      setParseError(null)
    } catch (e) {
      setParseError((e as Error).message)
    }
  }

  const handleCreate = async () => {
    try {
      const parsed = JSON.parse(jsonValue)
      await onCreate(parsed)
    } catch (e) {
      setParseError((e as Error).message)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg w-full max-w-2xl mx-4 overflow-hidden">
        <div className="px-4 py-3 border-b border-gray-700 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-lg font-medium text-white">Create New Entity</span>
            <select
              value={selectedType}
              onChange={(e) => onTypeChange(e.target.value)}
              className="bg-gray-700 text-gray-200 px-3 py-1 rounded text-sm border border-gray-600 focus:outline-none focus:border-blue-500"
            >
              {entityTypes.map((type) => (
                <option key={type} value={type}>
                  {type}
                </option>
              ))}
            </select>
          </div>
          <div className="flex gap-2">
            <button
              onClick={onCancel}
              disabled={creating}
              className="px-4 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={!!parseError || creating}
              className="px-4 py-1.5 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors disabled:opacity-50"
            >
              {creating ? 'Creating...' : 'Create'}
            </button>
          </div>
        </div>
        {(parseError || error) && (
          <div className="bg-red-900/30 border-b border-red-800 px-4 py-3 text-sm text-red-300">
            <div className="font-medium text-red-200 mb-1">
              {parseError ? 'JSON Parse Error' : 'Failed to create entity'}
            </div>
            <div className="text-red-400">
              {parseError || error}
            </div>
          </div>
        )}
        <Editor
          height="400px"
          defaultLanguage="json"
          value={jsonValue}
          theme="vs-dark"
          options={{
            minimap: { enabled: false },
            fontSize: 13,
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            wordWrap: 'on',
            automaticLayout: true,
          }}
          onChange={handleChange}
        />
        <div className="px-4 py-3 border-t border-gray-700 text-xs text-gray-500">
          Enter the entity JSON. The "type" field is required. The "id" and "last_updated" fields will be assigned by the server.
        </div>
      </div>
    </div>
  )
}
