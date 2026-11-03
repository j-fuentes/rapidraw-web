import { Layers, X } from 'lucide-react';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { useLibraryStore } from '../../store/useLibraryStore';
import { ImmichStackBadge as StackInfo } from '../ui/AppProperties';
import { toggleImmichStack } from './immichStacks';

interface Props {
  path: string;
  stack: StackInfo;
  isRaw: boolean;
  /** 'grid' (library thumbnails), 'strip' (filmstrip) or 'inline' (list view). */
  variant?: 'grid' | 'strip' | 'inline';
}

const stop = (event: React.SyntheticEvent) => event.stopPropagation();

/**
 * Marks a library entry that stands for an Immich stack (layers icon and the
 * number of files). Tapping it shows the other files of the stack right after
 * it, tapping again (on it or on any of those files) hides them. While the
 * stack is shown, each file says whether it is the cover and/or a RAW.
 */
export default function ImmichStackBadge({ path, stack, isRaw, variant = 'grid' }: Props) {
  const { t } = useTranslation();
  const entry = stack.parent ?? path;
  const expanded = useLibraryStore((state) => !!state.expandedImmichStacks[entry]);
  const showRoles = expanded || !!stack.parent;
  const label = expanded
    ? t('library.items.immichStackCollapse', { count: stack.count })
    : t('library.items.immichStackExpand', { count: stack.count });

  const chipClass = clsx(
    'rounded-full font-semibold leading-none shadow-md uppercase tracking-wide',
    variant === 'strip' ? 'px-1 py-0.5 text-[8px]' : 'px-1.5 py-1 text-[9px]',
  );

  return (
    <div
      className={clsx(
        'flex items-start pointer-events-none',
        variant === 'inline' ? 'flex-row items-center gap-1' : 'flex-col gap-1',
      )}
    >
      <button
        type="button"
        className={clsx(
          'pointer-events-auto flex items-center gap-1 rounded-full shadow-md font-semibold leading-none cursor-pointer select-none touch-manipulation transition-colors duration-150',
          variant === 'grid' && 'h-6 px-2 text-[11px] pointer-coarse:h-8 pointer-coarse:px-2.5 pointer-coarse:text-xs',
          variant === 'strip' && 'h-5 px-1.5 text-[10px] pointer-coarse:h-6 pointer-coarse:px-2',
          variant === 'inline' && 'h-5 px-1.5 text-[10px] pointer-coarse:h-7 pointer-coarse:px-2',
          expanded ? 'bg-accent text-button-text' : 'bg-black/60 text-white hover:bg-black/80',
        )}
        onClick={(event) => {
          event.stopPropagation();
          event.preventDefault();
          toggleImmichStack(entry);
        }}
        onPointerDown={stop}
        onMouseDown={stop}
        onTouchStart={stop}
        onDoubleClick={stop}
        onKeyDown={stop}
        aria-expanded={expanded}
        aria-label={label}
        data-tooltip={label}
        data-immich-stack={entry}
      >
        <Layers size={variant === 'grid' ? 13 : 11} className="shrink-0" />
        <span>{stack.count}</span>
        {expanded && <X size={variant === 'grid' ? 12 : 10} className="shrink-0 -mr-0.5" />}
      </button>
      {showRoles && (stack.is_primary || isRaw) && (
        <div className="flex items-center gap-1">
          {stack.is_primary && (
            <span
              className={clsx(chipClass, 'bg-white/90 text-black')}
              data-tooltip={t('library.items.immichStackCoverTooltip')}
            >
              {t('library.items.immichStackCover')}
            </span>
          )}
          {isRaw && <span className={clsx(chipClass, 'bg-black/60 text-white')}>RAW</span>}
        </div>
      )}
    </div>
  );
}
