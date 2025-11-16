import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Database, BookOpen, FileText } from 'lucide-react';
import { RagContextEntry } from '@/types/regulatory';

interface RagContextViewerProps {
  ragContext: RagContextEntry[];
}

export function RagContextViewer({ ragContext }: RagContextViewerProps) {
  // Sort by similarity (highest first)
  const sortedContext = [...ragContext].sort((a, b) => b.similarity - a.similarity);

  return (
    <Card className="border-2 border-blue-200 bg-gradient-to-r from-blue-50 to-indigo-50">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-blue-900">
          <Database className="h-5 w-5" />
          Regulatory Context Used (RAG)
        </CardTitle>
        <p className="text-sm text-blue-700">
          AI retrieved {ragContext.length} relevant regulatory sections from the knowledge base
        </p>
      </CardHeader>
      <CardContent className="space-y-3">
        {sortedContext.map((entry, idx) => (
          <div
            key={idx}
            className="border-l-4 border-blue-500 pl-4 py-3 bg-white rounded-r-lg shadow-sm hover:shadow-md transition-shadow"
          >
            <div className="flex justify-between items-start gap-4 mb-2">
              <div className="flex items-center gap-2">
                <BookOpen className="h-4 w-4 text-blue-600" />
                <span className="font-semibold text-gray-900">{entry.regulation_source}</span>
              </div>
              <Badge
                variant="outline"
                className={`${
                  entry.similarity >= 0.9
                    ? 'bg-green-100 text-green-800 border-green-300'
                    : entry.similarity >= 0.8
                    ? 'bg-blue-100 text-blue-800 border-blue-300'
                    : 'bg-gray-100 text-gray-800 border-gray-300'
                }`}
              >
                {(entry.similarity * 100).toFixed(1)}% match
              </Badge>
            </div>

            <div className="flex items-start gap-2 mb-2">
              <FileText className="h-4 w-4 text-gray-500 mt-0.5" />
              <div className="text-sm">
                <span className="font-medium text-gray-800">{entry.regulation_section}</span>
                {entry.section_title && (
                  <span className="text-gray-600"> - {entry.section_title}</span>
                )}
              </div>
            </div>

            {entry.content && (
              <div className="mt-2 pl-6">
                <p className="text-xs text-gray-600 line-clamp-2 italic">
                  &quot;{entry.content}&quot;
                </p>
              </div>
            )}
          </div>
        ))}

        <div className="mt-4 p-3 bg-blue-100 rounded-lg border border-blue-200">
          <p className="text-sm text-blue-900">
            <strong>AI + Regulatory Expertise:</strong> These FDA/EU/ICH regulations were semantically matched
            to your document requirements and used to generate compliant content.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
