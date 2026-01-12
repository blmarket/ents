import Editor from '@monaco-editor/react'
import type { Entity } from '../api/types'

interface EntityViewerProps {
  entity: Entity
  editable?: boolean
  onChange?: (value: string) => void
}

export default function EntityViewer({ entity, editable = false, onChange }: EntityViewerProps) {
  const jsonString = JSON.stringify(entity, null, 2)

  return (
    <div className="border border-gray-700 rounded overflow-hidden">
      <div className="bg-gray-800 px-3 py-2 border-b border-gray-700 flex items-center gap-2">
        <span className="text-sm font-medium text-gray-300">
          {entity.type}
        </span>
        <span className="text-xs text-gray-500">ID: {entity.id}</span>
        <span className="text-xs text-gray-500">
          Updated: {new Date(entity.last_updated * 1000).toLocaleString()}
        </span>
      </div>
      <Editor
        height="300px"
        defaultLanguage="json"
        value={jsonString}
        theme="vs-dark"
        options={{
          readOnly: !editable,
          minimap: { enabled: false },
          fontSize: 13,
          lineNumbers: 'on',
          scrollBeyondLastLine: false,
          wordWrap: 'on',
          automaticLayout: true,
        }}
        onChange={(value) => onChange?.(value || '')}
      />
    </div>
  )
}
