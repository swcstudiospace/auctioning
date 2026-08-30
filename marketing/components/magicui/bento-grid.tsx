import { type ComponentPropsWithoutRef, type ElementType, type ReactNode } from "react";
import Link from "next/link";
import { ArrowRight } from "lucide-react";

import { cn } from "@/lib/utils";

interface BentoGridProps extends ComponentPropsWithoutRef<"div"> {
  children: ReactNode;
  className?: string;
}

interface BentoCardProps extends ComponentPropsWithoutRef<"div"> {
  name: string;
  className: string;
  background: ReactNode;
  Icon: ElementType;
  description: string;
  href: string;
  cta: string;
}

const BentoGrid = ({ children, className, ...props }: BentoGridProps) => {
  return (
    <div
      className={cn("grid w-full auto-rows-[22rem] grid-cols-1 gap-4 md:grid-cols-3", className)}
      {...props}
    >
      {children}
    </div>
  );
};

const BentoCard = ({
  name,
  className,
  background,
  Icon,
  description,
  href,
  cta,
  ...props
}: BentoCardProps) => (
  <div
    className={cn(
      "group relative col-span-1 flex flex-col justify-between overflow-hidden rounded-2xl bg-white",
      "shadow-[0_1px_2px_rgba(15,40,25,0.04),0_12px_24px_rgba(15,40,25,0.05)]",
      className
    )}
    {...props}
  >
    <div>{background}</div>
    <div className="pointer-events-none z-10 flex transform-gpu flex-col gap-1 p-6 transition-all duration-300 lg:group-hover:-translate-y-8">
      <Icon className="h-10 w-10 origin-left text-forest transition-all duration-300 ease-in-out group-hover:scale-75" />
      <h3 className="text-xl font-semibold text-ink">{name}</h3>
      <p className="max-w-lg text-sm leading-relaxed text-muted">{description}</p>
    </div>
    <div className="absolute bottom-0 hidden w-full translate-y-8 p-6 opacity-0 transition-all duration-300 group-hover:translate-y-0 group-hover:opacity-100 lg:flex">
      <Link href={href} className="pointer-events-auto inline-flex items-center gap-1 text-sm font-medium text-forest">
        {cta}
        <ArrowRight className="h-4 w-4" />
      </Link>
    </div>
    <div className="p-6 pt-0 lg:hidden">
      <Link href={href} className="inline-flex items-center gap-1 text-sm font-medium text-forest">
        {cta}
        <ArrowRight className="h-4 w-4" />
      </Link>
    </div>
  </div>
);

export { BentoCard, BentoGrid };
