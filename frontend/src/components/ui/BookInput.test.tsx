import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { BookInput, BookTextarea } from './BookInput';

describe('BookInput accessibility contract', () => {
  it('associates labels and errors with the input', () => {
    const html = renderToStaticMarkup(
      <BookInput id="project-title" label="项目标题" error="标题不能为空" />,
    );

    expect(html).toContain('for="project-title"');
    expect(html).toContain('aria-invalid="true"');
    expect(html).toContain('aria-describedby="project-title-error"');
    expect(html).toContain('id="project-title-error"');
  });

  it('preserves caller descriptions while linking textarea errors', () => {
    const html = renderToStaticMarkup(
      <BookTextarea
        id="story-summary"
        label="故事摘要"
        error="摘要不能为空"
        aria-describedby="story-summary-help"
      />,
    );

    expect(html).toContain(
      'aria-describedby="story-summary-help story-summary-error"',
    );
  });
});
