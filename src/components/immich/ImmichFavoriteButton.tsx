import { Heart } from 'lucide-react';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { setImmichFavorites } from './immichFavorites';

interface Props {
  path: string;
  favorite: boolean;
  /** 'grid' (library thumbnails), 'strip' (filmstrip) or 'inline' (list view). */
  variant?: 'grid' | 'strip' | 'inline';
}

const stop = (event: React.SyntheticEvent) => event.stopPropagation();

/**
 * Heart of an image marked as favorite in Immich. Tapping it unmarks the image; on images that are not
 * favorites an outlined heart shows on hover (mouse only) to mark them. Same look as the stack badge.
 */
export default function ImmichFavoriteButton({ path, favorite, variant = 'grid' }: Props) {
  const { t } = useTranslation();
  const label = favorite ? t('library.items.immichFavoriteRemove') : t('library.items.immichFavoriteAdd');
  const iconSize = variant === 'grid' ? 13 : 11;

  return (
    <button
      type="button"
      className={clsx(
        'pointer-events-auto flex items-center justify-center rounded-full shadow-md cursor-pointer select-none touch-manipulation transition-all duration-150 bg-black/60 text-white hover:bg-black/80',
        variant === 'grid' && 'h-6 w-6 pointer-coarse:h-8 pointer-coarse:w-8',
        variant === 'strip' && 'h-5 w-5 pointer-coarse:h-6 pointer-coarse:w-6',
        variant === 'inline' && 'h-5 w-5 shrink-0 pointer-coarse:h-7 pointer-coarse:w-7',
        // Not a favorite: only on hover with a mouse, never on touch screens.
        !favorite && 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100 pointer-coarse:hidden',
      )}
      onClick={(event) => {
        event.stopPropagation();
        event.preventDefault();
        setImmichFavorites([path], !favorite);
      }}
      onPointerDown={stop}
      onMouseDown={stop}
      onTouchStart={stop}
      onDoubleClick={stop}
      onKeyDown={stop}
      aria-pressed={favorite}
      aria-label={label}
      data-tooltip={label}
      data-immich-favorite={favorite ? 'true' : 'false'}
    >
      <Heart size={iconSize} className={clsx('shrink-0', favorite && 'fill-current')} />
    </button>
  );
}
