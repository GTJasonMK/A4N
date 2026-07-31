import type { ChapterData } from "../../../api/chapter";

export interface StoryEvent {
  id: string;
  chapterId: number;
  chapterNumber: number;
  chapterTitle: string;
  description: string;
  type: string;
  importance: string;
  involvedCharacters: string[];
}

export interface ChapterStorySection {
  chapter: ChapterData;
  title: string;
  summary: string;
  timelineMarker: string;
  tone: string;
  characters: string[];
  locations: string[];
  keywords: string[];
  events: StoryEvent[];
  plantedCount: number;
  resolvedCount: number;
  tensionCount: number;
  hasAnalysis: boolean;
}

export interface StoryEventFilters {
  query: string;
  character: string;
  type: string;
  importance: string;
}

export interface StoryEventFilterOptions {
  characters: string[];
  eventTypes: string[];
  importances: string[];
}
