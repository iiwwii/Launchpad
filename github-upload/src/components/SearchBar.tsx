interface Props {
  value: string;
  onChange: (v: string) => void;
}

export default function SearchBar({ value, onChange }: Props) {
  return (
    <div
      className="search-wrap"
      onMouseDown={(e) => e.stopPropagation()}
    >
      <input
        className="search-input"
        placeholder="搜索"
        value={value}
        autoFocus
        spellCheck={false}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}
