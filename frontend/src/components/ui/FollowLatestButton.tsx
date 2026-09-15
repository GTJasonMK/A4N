import { ArrowDown } from "lucide-react";
import { BookIconButton } from "./BookButton";

interface FollowLatestButtonProps {
  isFollowingLatest: boolean;
  hasNewContent: boolean;
  newContentLabel: string;
  latestLabel: string;
  onClick: () => void;
  className?: string;
}

export function FollowLatestButton({
  isFollowingLatest,
  hasNewContent,
  newContentLabel,
  latestLabel,
  onClick,
  className = "",
}: FollowLatestButtonProps) {
  if (isFollowingLatest) return null;
  return (
    <BookIconButton
      size="md"
      variant="secondary"
      label={hasNewContent ? newContentLabel : latestLabel}
      className={`shadow-md ${className}`}
      onClick={onClick}
    >
      <ArrowDown size={15} aria-hidden="true" />
    </BookIconButton>
  );
}
